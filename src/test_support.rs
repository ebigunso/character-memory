// Deterministic test harness shared by pipeline, adapter, and facade tests.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, MutexGuard,
};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
use crate::adapters::qdrant_edge::QdrantEdgeVectorCandidateStore;
use crate::adapters::stats::InMemoryRetrievalStatsStore;
use crate::api::types::VectorRecallCompleteness;
use crate::domain::ScopeKey;
use crate::domain::{
    DerivedMemory, DerivedType, Entity, Episode, MemoryId, MemoryLink, MemoryObject,
    MemoryObjectRef, MemoryThread, Modality, ObjectType, Observation, RelationType, RetentionState,
    ThreadStatus, DEFAULT_SCHEMA_VERSION,
};
use crate::errors::{
    CustomError, GraphQueryError, RetrievalStatsHealthCause, RetrievalStatsStoreError,
    VectorDatabaseError, VectorDatabaseErrorKind,
};
use crate::models::vector::{
    CanonicalCandidates, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
    VectorRecordEmbedding,
};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::*;
use crate::ports::retrieval_stats::*;
use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StoreCall {
    GraphQuery(Vec<MemoryId>),
    GraphThreadQuery(Vec<MemoryId>),
    GraphObjects(Vec<MemoryId>),
    GraphLinks(Vec<MemoryLink>),
    EmbedBatch(Vec<MemoryId>),
    VectorUpsert(Vec<MemoryId>),
    VectorDelete(Vec<MemoryObjectRef>),
    StatsEdges(usize),
    StatsObjectStates(usize),
    StatsUnhealthy,
}

pub(crate) struct TemporaryVectorCandidateStore {
    store: Option<QdrantEdgeVectorCandidateStore>,
    directory: tempfile::TempDir,
    pub(crate) calls: Arc<Mutex<Vec<StoreCall>>>,
    pub(crate) upsert_error: Option<&'static str>,
    pub(crate) delete_error: Option<&'static str>,
    pub(crate) delete_error_once: Option<AtomicBool>,
    pub(crate) upsert_hook: Option<fn()>,
    pub(crate) gate: Option<Arc<Gate>>,
    pub(crate) completeness: Option<VectorRecallCompleteness>,
    pub(crate) candidate: Option<VectorCandidateMatch>,
}

impl TemporaryVectorCandidateStore {
    pub(crate) async fn open(vector_size: usize) -> Self {
        let directory = tempfile::TempDir::new().expect("temporary vector directory");
        let store = QdrantEdgeVectorCandidateStore::open(
            directory.path(),
            format!("test_{}", Uuid::new_v4().simple()),
            vector_size,
        )
        .await
        .expect("temporary embedded vector store");
        Self {
            store: Some(store),
            directory,
            calls: Arc::default(),
            upsert_error: None,
            delete_error: None,
            delete_error_once: None,
            upsert_hook: None,
            gate: None,
            completeness: None,
            candidate: None,
        }
    }

    pub(crate) fn store(&self) -> &QdrantEdgeVectorCandidateStore {
        self.store.as_ref().expect("temporary vector store is open")
    }
    pub(crate) fn with_calls(mut self, calls: Arc<Mutex<Vec<StoreCall>>>) -> Self {
        self.calls = calls;
        self
    }

    pub(crate) fn fail_upsert(mut self, message: &'static str) -> Self {
        self.upsert_error = Some(message);
        self
    }

    pub(crate) fn fail_delete(mut self, message: &'static str) -> Self {
        self.delete_error = Some(message);
        self
    }

    pub(crate) fn fail_delete_once(mut self) -> Self {
        self.delete_error = Some("one-shot vector delete failed");
        self.delete_error_once = Some(AtomicBool::new(true));
        self
    }

    pub(crate) fn calls(&self) -> Vec<StoreCall> {
        lock(&self.calls).clone()
    }
}

impl Drop for TemporaryVectorCandidateStore {
    fn drop(&mut self) {
        // During unwinding the shutdown is best effort: a second panic here
        // would abort the process and hide the test's own failure.
        let unwinding = std::thread::panicking();
        let Some(store) = self.store.take() else {
            assert!(unwinding, "temporary vector store is open");
            return;
        };
        // Fallible spawn: creating the shutdown thread can itself fail, and
        // that must not panic while the test is already unwinding.
        let shutdown = std::thread::Builder::new()
            .name("temporary-vector-shutdown".into())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .build()
                    .map_err(|error| error.to_string())?
                    .block_on(store.close())
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.to_string())
            .and_then(|handle| {
                handle
                    .join()
                    .map_err(|_| "temporary vector shutdown thread panicked".to_string())
            })
            .and_then(|result| result);
        if !unwinding {
            shutdown.expect("temporary vector store shutdown");
        }
    }
}

#[async_trait]
impl VectorCandidateStore for TemporaryVectorCandidateStore {
    async fn close(&self) -> Result<(), CustomError> {
        self.store().close().await
    }

    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        if let Some(gate) = &self.gate {
            gate.stop_once().await;
        }
        if let Some(hook) = self.upsert_hook {
            hook();
        }
        lock(&self.calls).push(StoreCall::VectorUpsert(
            records
                .iter()
                .map(|record| record.record.object_id)
                .collect(),
        ));
        if let Some(message) = self.upsert_error {
            return Err(vector_error(message));
        }
        self.store().upsert_vector_records(records).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        let mut recall = self.store().search_candidates(query).await?;
        if let Some(completeness) = self.completeness {
            recall.completeness = completeness;
        }
        if let Some(candidate) = &self.candidate {
            recall.candidates = CanonicalCandidates::new([candidate.clone()]);
        }
        Ok(recall)
    }

    async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
        lock(&self.calls).push(StoreCall::VectorDelete(objects.to_vec()));
        if let Some(message) = self.delete_error {
            if self
                .delete_error_once
                .as_ref()
                .is_none_or(|once| once.swap(false, Ordering::SeqCst))
            {
                return Err(vector_error(message));
            }
        }
        self.store().delete_candidates(objects).await
    }
}

#[derive(Default)]
pub(crate) struct Gate {
    armed: AtomicBool,
    pub(crate) entered: Notify,
    pub(crate) release: Notify,
}

impl Gate {
    pub(crate) fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn stop_once(&self) {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
    }
}

pub(crate) struct TestGraphStore {
    pub(crate) store: OxigraphGraphAuthorityStore,
    pub(crate) calls: Arc<Mutex<Vec<StoreCall>>>,
    pub(crate) fail_objects: bool,
    pub(crate) fail_links: bool,
    pub(crate) fail_id_queries: bool,
    pub(crate) fail_currency_query: bool,
    pub(crate) query_error: Option<GraphQueryError>,
    pub(crate) expansion_error: Option<GraphQueryError>,
    pub(crate) record_queries: bool,
    pub(crate) gate: Option<Arc<Gate>>,
}

impl Default for TestGraphStore {
    fn default() -> Self {
        Self {
            store: in_memory_graph_store(),
            calls: Arc::default(),
            fail_objects: false,
            fail_links: false,
            fail_id_queries: false,
            fail_currency_query: false,
            query_error: None,
            expansion_error: None,
            record_queries: false,
            gate: None,
        }
    }
}

impl TestGraphStore {
    pub(crate) fn fail_objects(mut self) -> Self {
        self.fail_objects = true;
        self
    }

    pub(crate) fn fail_links(mut self) -> Self {
        self.fail_links = true;
        self
    }

    pub(crate) async fn with_query_objects(self, objects: Vec<MemoryObject>) -> Self {
        self.store.upsert_objects(&objects).await.unwrap();
        self
    }

    pub(crate) fn fail_id_queries(mut self) -> Self {
        self.fail_id_queries = true;
        self
    }

    pub(crate) fn calls(&self) -> Vec<StoreCall> {
        lock(&self.calls).clone()
    }
}

#[async_trait]
impl GraphAuthorityStore for TestGraphStore {
    async fn ensure_character_identity(&self, character_id: MemoryId) -> Result<(), CustomError> {
        self.store.ensure_character_identity(character_id).await
    }

    async fn query_anniversaries(
        &self,
        date: chrono::NaiveDate,
        participants: &[crate::domain::MemoryId],
        limit: usize,
        policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<(crate::ports::graph_authority::GraphMemoryRank, bool)>, CustomError> {
        self.store
            .query_anniversaries(date, participants, limit, policy)
            .await
    }

    async fn query_episodes_by_time(
        &self,
        start: Option<chrono::DateTime<chrono::Utc>>,
        end: chrono::DateTime<chrono::Utc>,
        limit: usize,
        policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<crate::ports::graph_authority::GraphMemoryRank>, CustomError> {
        self.store
            .query_episodes_by_time(start, end, limit, policy)
            .await
    }

    async fn query_episode_occasions(
        &self,
        episodes: &[crate::domain::MemoryObjectRef],
    ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError> {
        self.store.query_episode_occasions(episodes).await
    }

    async fn query_last_interaction(
        &self,
        participant: MemoryId,
        reference_time: chrono::DateTime<chrono::Utc>,
        policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
    ) -> Result<Option<(MemoryId, chrono::DateTime<chrono::Utc>)>, CustomError> {
        self.store
            .query_last_interaction(participant, reference_time, policy)
            .await
    }

    async fn query_notions_known_as(
        &self,
        name: &str,
    ) -> Result<Vec<MemoryId>, crate::errors::GraphQueryError> {
        self.store.query_notions_known_as(name).await
    }

    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        lock(&self.calls).push(StoreCall::GraphObjects(
            objects.iter().map(MemoryObject::id).collect(),
        ));
        if self.fail_objects {
            return Err(CustomError::DatabaseError("object write failed".to_owned()));
        }
        self.store.upsert_objects(objects).await
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        lock(&self.calls).push(StoreCall::GraphLinks(links.to_vec()));
        if self.fail_links {
            return Err(CustomError::DatabaseError("link write failed".to_owned()));
        }
        self.store.upsert_links(links).await
    }

    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        if let Some(gate) = &self.gate {
            gate.stop_once().await;
        }
        lock(&self.calls).push(StoreCall::GraphObjects(
            objects.iter().map(MemoryObject::id).collect(),
        ));
        if self.fail_objects {
            return Err(CustomError::DatabaseError("object write failed".to_owned()));
        }
        lock(&self.calls).push(StoreCall::GraphLinks(links.to_vec()));
        if self.fail_links {
            return Err(CustomError::DatabaseError("link write failed".to_owned()));
        }
        self.store.upsert_objects_and_links(objects, links).await
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, crate::errors::GraphQueryError> {
        if self.record_queries {
            let ids = match query {
                GraphObjectQuery::ByRefs(refs) => refs.iter().map(|object| object.id).collect(),
                GraphObjectQuery::ByIds(ids) => ids.clone(),
                GraphObjectQuery::ByTypes { .. } => Vec::new(),
            };
            lock(&self.calls).push(StoreCall::GraphQuery(ids));
        }
        if let Some(error) = &self.query_error {
            return Err(error.clone());
        }
        if self.fail_id_queries && matches!(query, GraphObjectQuery::ByIds(_)) {
            return Err(crate::errors::GraphQueryError::Selection {
                detail: "endpoint lifecycle lookup failed".to_owned(),
            });
        }

        self.store.query_objects(query).await
    }

    async fn query_superseded_derived_memory_ids(
        &self,
        memory_ids: &[crate::domain::MemoryId],
    ) -> Result<Vec<crate::domain::MemoryId>, crate::errors::GraphQueryError> {
        if self.fail_currency_query {
            return Err(crate::errors::GraphQueryError::Selection {
                detail: "currency lookup failed".to_owned(),
            });
        }
        self.store
            .query_superseded_derived_memory_ids(memory_ids)
            .await
    }

    async fn query_links_by_ids(
        &self,
        link_ids: &[MemoryId],
    ) -> Result<Vec<MemoryLink>, CustomError> {
        self.store.query_links_by_ids(link_ids).await
    }

    async fn query_derived_memories_by_provenance(
        &self,
        query: &crate::ports::graph_authority::GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
        self.store.query_derived_memories_by_provenance(query).await
    }

    async fn query_derived_memories_by_thread(
        &self,
        query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
    ) -> Result<
        (
            Vec<crate::domain::DerivedMemory>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        if self.record_queries {
            lock(&self.calls).push(StoreCall::GraphThreadQuery(query.thread_ids.clone()));
        }
        self.store.query_derived_memories_by_thread(query).await
    }

    async fn query_thread_state(
        &self,
        query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<crate::ports::graph_authority::GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        self.store.query_thread_state(query, limit).await
    }

    async fn query_scope_state(
        &self,
        key: &ScopeKey,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        self.store.query_scope_state(key, policy, limit).await
    }

    async fn query_party_obligations(
        &self,
        party: MemoryId,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        self.store
            .query_party_obligations(party, policy, limit)
            .await
    }

    async fn query_due_obligations(
        &self,
        before: DateTime<Utc>,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        self.store
            .query_due_obligations(before, policy, limit)
            .await
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        if let Some(error) = &self.expansion_error {
            return Err(error.clone().into());
        }
        self.store.expand_bounded(query).await
    }
}

pub(crate) enum StatsRead<'a> {
    Counter(&'a RetrievalStatsCounterKey),
    GlobalCounter(RelationType),
    GlobalEpisodes,
    Health,
}

type StatsReadHook =
    Box<dyn for<'a> Fn(StatsRead<'a>) -> Result<(), RetrievalStatsStoreError> + Send + Sync>;

#[derive(Default)]
pub(crate) struct TestStatsStore {
    pub(crate) store: InMemoryRetrievalStatsStore,
    pub(crate) calls: Arc<Mutex<Vec<StoreCall>>>,
    pub(crate) edge_error: Option<RetrievalStatsStoreError>,
    pub(crate) edge_error_once: Option<AtomicBool>,
    pub(crate) object_state_error: Option<RetrievalStatsStoreError>,
    pub(crate) health_error: Option<RetrievalStatsStoreError>,
    pub(crate) before_read: Option<StatsReadHook>,
    pub(crate) marked_causes: Mutex<Vec<RetrievalStatsHealthCause>>,
    pub(crate) gate: Option<Arc<Gate>>,
    pub(crate) projected: Arc<Mutex<Vec<RetrievalStatsObjectState>>>,
}

impl TestStatsStore {
    fn check_read(&self, call: StatsRead<'_>) -> Result<(), RetrievalStatsStoreError> {
        if let Some(hook) = &self.before_read {
            hook(call)?;
        }
        Ok(())
    }
}

#[async_trait]
impl RetrievalStatsStore for TestStatsStore {
    async fn record_edges(
        &self,
        edges: &[RetrievalStatsEdge],
    ) -> Result<(), RetrievalStatsStoreError> {
        lock(&self.calls).push(StoreCall::StatsEdges(edges.len()));
        if let Some(error) = &self.edge_error {
            if self
                .edge_error_once
                .as_ref()
                .is_none_or(|once| once.swap(false, Ordering::SeqCst))
            {
                return Err(error.clone());
            }
        }
        self.store.record_edges(edges).await
    }
    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), RetrievalStatsStoreError> {
        if let Some(gate) = &self.gate {
            gate.stop_once().await;
        }
        lock(&self.calls).push(StoreCall::StatsObjectStates(states.len()));
        if let Some(error) = &self.object_state_error {
            return Err(error.clone());
        }
        self.store.record_object_states(states).await?;
        lock(&self.projected).extend_from_slice(states);
        Ok(())
    }
    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        self.check_read(StatsRead::Counter(key))?;
        self.store.counter(key).await
    }
    async fn global_counter(
        &self,
        relation: RelationType,
        object_type: ObjectType,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        let _ = object_type;
        self.check_read(StatsRead::GlobalCounter(relation))?;
        self.store.global_counter(relation, object_type).await
    }
    async fn global_episode_counter(
        &self,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        self.check_read(StatsRead::GlobalEpisodes)?;
        self.store.global_episode_counter().await
    }
    async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
        self.check_read(StatsRead::Health)?;
        if let Some(error) = &self.health_error {
            return Err(error.clone());
        }
        self.store.health().await
    }
    async fn mark_unhealthy(
        &self,
        cause: RetrievalStatsHealthCause,
    ) -> Result<(), RetrievalStatsStoreError> {
        lock(&self.calls).push(StoreCall::StatsUnhealthy);
        lock(&self.marked_causes).push(cause.clone());
        self.store.mark_unhealthy(cause).await
    }
}

fn vector_error(message: &str) -> CustomError {
    CustomError::VectorDatabaseError(VectorDatabaseError::new(
        "test",
        VectorDatabaseErrorKind::Response,
        None,
        message,
    ))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("test mutex should not be poisoned")
}

pub(crate) fn in_memory_graph_store() -> OxigraphGraphAuthorityStore {
    OxigraphGraphAuthorityStore::new_in_memory().expect("in-memory graph store")
}

pub(crate) struct TestEmbedder<F>(pub(crate) F);

#[async_trait]
impl<F> MemoryEmbedder for TestEmbedder<F>
where
    F: Fn(&EmbeddingInput) -> Vec<f32> + Send + Sync,
{
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        Ok((self.0)(input))
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        Ok(inputs.iter().map(&self.0).collect())
    }
}

pub(crate) fn deterministic_embedder(dimensions: usize) -> impl MemoryEmbedder {
    TestEmbedder(move |input: &EmbeddingInput| deterministic_embedding(input, dimensions))
}

pub(crate) async fn memory_with_embedder(
    dimensions: usize,
    embedder: impl MemoryEmbedder + 'static,
    character_id: MemoryId,
) -> crate::CharacterMemory {
    crate::CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(dimensions).await),
        Box::new(embedder),
        character_id,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RepresentativeFixtures {
    pub(crate) episode: Episode,
    pub(crate) salient_observation: Observation,
    pub(crate) user_entity: Entity,
    pub(crate) assistant_entity: Entity,
    pub(crate) project_entity: Entity,
    pub(crate) hub_entity: Entity,
    pub(crate) soft_thread: MemoryThread,
    pub(crate) derived_reflection: DerivedMemory,
    pub(crate) user_preference: DerivedMemory,
    pub(crate) open_loop: DerivedMemory,
    pub(crate) commitment: DerivedMemory,
    pub(crate) correction: DerivedMemory,
    pub(crate) suppressed_seed: DerivedMemory,
    pub(crate) soft_thread_link: MemoryLink,
    pub(crate) hub_links: Vec<MemoryLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HighFanoutGraphFixture {
    pub(crate) hub_entity: Entity,
    pub(crate) episode: Episode,
    pub(crate) observation: Observation,
    pub(crate) derived_memories: Vec<DerivedMemory>,
    pub(crate) links: Vec<MemoryLink>,
}

impl HighFanoutGraphFixture {
    pub(crate) fn objects(&self) -> Vec<MemoryObject> {
        let mut objects = vec![
            MemoryObject::Entity(self.hub_entity.clone()),
            MemoryObject::Episode(self.episode.clone()),
            MemoryObject::Observation(self.observation.clone()),
        ];
        objects.extend(
            self.derived_memories
                .iter()
                .cloned()
                .map(MemoryObject::DerivedMemory),
        );
        objects
    }
}

impl RepresentativeFixtures {
    pub(crate) fn objects(&self) -> Vec<MemoryObject> {
        vec![
            MemoryObject::Episode(self.episode.clone()),
            MemoryObject::Observation(self.salient_observation.clone()),
            MemoryObject::Entity(self.user_entity.clone()),
            MemoryObject::Entity(self.assistant_entity.clone()),
            MemoryObject::Entity(self.project_entity.clone()),
            MemoryObject::Entity(self.hub_entity.clone()),
            MemoryObject::MemoryThread(self.soft_thread.clone()),
            MemoryObject::DerivedMemory(self.derived_reflection.clone()),
            MemoryObject::DerivedMemory(self.user_preference.clone()),
            MemoryObject::DerivedMemory(self.open_loop.clone()),
            MemoryObject::DerivedMemory(self.commitment.clone()),
            MemoryObject::DerivedMemory(self.correction.clone()),
            MemoryObject::DerivedMemory(self.suppressed_seed.clone()),
        ]
    }

    pub(crate) fn links(&self) -> Vec<MemoryLink> {
        let mut links = vec![self.soft_thread_link.clone()];
        links.extend(self.hub_links.clone());
        links
    }
}

pub(crate) fn representative_fixtures() -> RepresentativeFixtures {
    let episode = simple_episode();
    let salient_observation = salient_observation(episode.id, fixture_id(1));
    let user_entity = entity(fixture_id(1));
    let assistant_entity = entity(fixture_id(2));
    let project_entity = entity(fixture_id(3));
    let hub_entity = entity(fixture_id(4));
    let soft_thread = soft_thread();
    let derived_reflection = derived_memory(
        fixture_id(30),
        DerivedType::Reflection,
        "The user wants service-free contract tests before pipeline wiring.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![user_entity.id, project_entity.id],
        Vec::new(),
        RetentionState::Active,
    );
    let user_preference = derived_memory(
        fixture_id(31),
        DerivedType::UserPreference,
        "Prefer deterministic local fakes for contract-level tests.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![user_entity.id],
        Vec::new(),
        RetentionState::Active,
    );
    let open_loop = derived_memory(
        fixture_id(32),
        DerivedType::OpenLoop,
        "Add pipeline tests once store contracts have reusable fakes.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![project_entity.id],
        Vec::new(),
        RetentionState::Active,
    );
    let commitment = derived_memory(
        fixture_id(33),
        DerivedType::Commitment,
        "Complete Task_3 with service-free validation.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![assistant_entity.id, project_entity.id],
        Vec::new(),
        RetentionState::Active,
    );
    let correction = derived_memory(
        fixture_id(34),
        DerivedType::Correction,
        "Correction seed supersedes an outdated preference seed.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![user_entity.id],
        vec![fixture_id(35)],
        RetentionState::Active,
    );
    let suppressed_seed = derived_memory(
        fixture_id(35),
        DerivedType::UserPreference,
        "Suppressed seed retained to prove lifecycle preservation.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![user_entity.id],
        Vec::new(),
        RetentionState::Suppressed,
    );
    let soft_thread_link = link(
        fixture_id(50),
        salient_observation.id,
        ObjectType::Observation,
        soft_thread.id,
        ObjectType::MemoryThread,
        RelationType::PartOfThread,
    );
    let hub_links = vec![
        link(
            fixture_id(51),
            hub_entity.id,
            ObjectType::Entity,
            episode.id,
            ObjectType::Episode,
            RelationType::Involves,
        ),
        link(
            fixture_id(52),
            hub_entity.id,
            ObjectType::Entity,
            derived_reflection.id,
            ObjectType::DerivedMemory,
            RelationType::About,
        ),
        link(
            fixture_id(53),
            correction.id,
            ObjectType::DerivedMemory,
            suppressed_seed.id,
            ObjectType::DerivedMemory,
            RelationType::Supersedes,
        ),
        link(
            fixture_id(54),
            open_loop.id,
            ObjectType::DerivedMemory,
            commitment.id,
            ObjectType::DerivedMemory,
            RelationType::FulfillsCommitment,
        ),
    ];

    RepresentativeFixtures {
        episode,
        salient_observation,
        user_entity,
        assistant_entity,
        project_entity,
        hub_entity,
        soft_thread,
        derived_reflection,
        user_preference,
        open_loop,
        commitment,
        correction,
        suppressed_seed,
        soft_thread_link,
        hub_links,
    }
}

pub(crate) fn high_fanout_graph_fixture() -> HighFanoutGraphFixture {
    let episode = simple_episode();
    let observation = salient_observation(episode.id, fixture_id(1));
    let hub_entity = entity(fixture_id(90));
    let derived_memories = (0_u128..12)
        .map(|offset| {
            derived_memory(
                fixture_id(100 + offset),
                DerivedType::ProjectNote,
                format!("High fanout derived memory {offset}."),
                episode.id,
                observation.id,
                Vec::new(),
                vec![hub_entity.id],
                Vec::new(),
                RetentionState::Active,
            )
        })
        .collect::<Vec<_>>();
    let mut links = vec![
        link(
            fixture_id(190),
            hub_entity.id,
            ObjectType::Entity,
            episode.id,
            ObjectType::Episode,
            RelationType::Involves,
        ),
        link(
            fixture_id(191),
            hub_entity.id,
            ObjectType::Entity,
            observation.id,
            ObjectType::Observation,
            RelationType::Mentions,
        ),
    ];
    links.extend(
        derived_memories
            .iter()
            .rev()
            .enumerate()
            .map(|(index, memory)| {
                link(
                    fixture_id(200 + index as u128),
                    hub_entity.id,
                    ObjectType::Entity,
                    memory.id,
                    ObjectType::DerivedMemory,
                    RelationType::About,
                )
            }),
    );

    HighFanoutGraphFixture {
        hub_entity,
        episode,
        observation,
        derived_memories,
        links,
    }
}

pub(crate) fn simple_episode() -> Episode {
    Episode {
        id: fixture_id(10),
        object_type: ObjectType::Episode,
        modality: Modality::Chat,
        scene: crate::domain::Scene {
            setting: crate::domain::SceneSetting {
                key: Some("conversation:contract-fixture".to_owned()),
                words: None,
            },
            participants: vec![
                crate::domain::SceneParticipant {
                    key: Some(fixture_id(1)),
                    ..Default::default()
                },
                crate::domain::SceneParticipant {
                    key: Some(fixture_id(2)),
                    ..Default::default()
                },
            ],
            ..crate::domain::Scene::at((timestamp("2026-04-27T10:00:00Z")).fixed_offset())
        },
        ended_at: Some(timestamp("2026-04-27T10:10:00Z")),
        summary: "Discussed deterministic store contract fixtures.".to_owned(),
        raw_ref: Some("file:fixtures/raw/simple-episode.txt".to_owned()),
        salience_score: 0.8,
        retention_state: RetentionState::Active,
        created_at: timestamp("2026-04-27T10:11:00Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

pub(crate) fn salient_observation(
    episode_id: MemoryId,
    speaker_entity_id: MemoryId,
) -> Observation {
    Observation {
        id: fixture_id(20),
        object_type: ObjectType::Observation,
        episode_id,
        speaker_entity_id: Some(speaker_entity_id),
        observed_at: Some(timestamp("2026-04-27T10:03:00Z")),
        modality: Modality::Chat,
        text: "Use deterministic fakes instead of service-backed stores.".to_owned(),
        raw_ref: Some("file:fixtures/raw/salient-observation.txt".to_owned()),
        salience_score: 0.9,
        retention_state: RetentionState::Active,
        created_at: timestamp("2026-04-27T10:11:01Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn entity(id: MemoryId) -> Entity {
    Entity {
        id,
        object_type: ObjectType::Entity,
        created_at: timestamp("2026-04-27T10:11:02Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn soft_thread() -> MemoryThread {
    MemoryThread {
        id: fixture_id(25),
        object_type: ObjectType::MemoryThread,
        title: "Contract test support".to_owned(),
        summary: "Soft thread connecting store-contract fixture objects.".to_owned(),
        status: ThreadStatus::Active,
        last_touched_at: timestamp("2026-04-27T10:11:04Z"),
        salience_score: 0.7,
        canonical_key: Some("thread:contract-test-support".to_owned()),
        created_at: timestamp("2026-04-27T10:11:02Z"),
        updated_at: timestamp("2026-04-27T10:11:04Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn derived_memory(
    id: MemoryId,
    derived_type: DerivedType,
    text: impl Into<String>,
    episode_id: MemoryId,
    observation_id: MemoryId,
    thread_ids: Vec<MemoryId>,
    entity_ids: Vec<MemoryId>,
    supersedes: Vec<MemoryId>,
    retention_state: RetentionState,
) -> DerivedMemory {
    DerivedMemory {
        scope_keys: Vec::new(),
        assertions: Vec::new(),
        due_at: None,
        given_by_application: false,
        id,
        object_type: ObjectType::DerivedMemory,
        derived_type,
        text: text.into(),
        derived_from_episode_ids: vec![episode_id],
        derived_from_observation_ids: vec![observation_id],
        thread_ids,
        entity_ids,
        salience_score: 0.75,
        supersedes,
        retention_state,
        created_at: timestamp("2026-04-27T10:11:05Z"),
        updated_at: timestamp("2026-04-27T10:11:06Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn link(
    id: MemoryId,
    from_id: MemoryId,
    from_type: ObjectType,
    to_id: MemoryId,
    to_type: ObjectType,
    relation: RelationType,
) -> MemoryLink {
    MemoryLink {
        id,
        object_type: ObjectType::MemoryLink,
        from_id,
        from_type,
        to_id,
        to_type,
        relation,
        rationale: Some("Representative fixture link.".to_owned()),
        created_at: timestamp("2026-04-27T10:11:07Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn fixture_id(suffix: u128) -> MemoryId {
    Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0000 + suffix)
}

pub(crate) fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn deterministic_embedding(input: &EmbeddingInput, dimensions: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; dimensions];
    if dimensions == 0 {
        return embedding;
    }

    let seed = format!("{:?}|{:?}|{}", input.object_type, input.surface, input.text);
    for (index, byte) in seed.bytes().enumerate() {
        let slot = index % dimensions;
        let signed = (byte as f32 / 255.0) - 0.5;
        embedding[slot] += signed;
    }

    let magnitude = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if magnitude > 0.0 {
        for value in &mut embedding {
            *value /= magnitude;
        }
    }

    embedding
}

pub(crate) fn parse_id(value: &str) -> MemoryId {
    MemoryId::parse_str(value).unwrap()
}
pub(crate) fn write_time() -> DateTime<Utc> {
    timestamp("2026-04-28T12:00:00Z")
}

pub(crate) fn pack_contains_derived_memory(
    pack: &crate::ContinuityContextPack,
    memory_id: MemoryId,
) -> bool {
    pack.derived_memories
        .iter()
        .chain(pack.preferences.iter())
        .chain(pack.relationship_notes.iter())
        .chain(pack.open_loops.iter())
        .chain(pack.commitments.iter())
        .chain(pack.character_signals.iter())
        .any(|included| included.memory.id == memory_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn temporary_vector_store_removes_its_directory_on_drop() {
        let store = TemporaryVectorCandidateStore::open(2).await;
        let path = store.directory.path().to_path_buf();
        assert!(path.exists());

        drop(store);

        assert!(!path.exists());
    }
}
