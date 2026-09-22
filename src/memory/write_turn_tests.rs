use std::future::Future;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;

use crate::adapters::{oxigraph::OxigraphGraphAuthorityStore, stats::InMemoryRetrievalStatsStore};
use crate::api::types::*;
use crate::domain::*;
use crate::errors::{GraphQueryError, RetrievalStatsHealthCause, RetrievalStatsStoreError};
use crate::models::vector::{EmbeddingInput, VectorCandidateSearch, VectorRecordEmbedding};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::*;
use crate::ports::retrieval_stats::*;
use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};
use crate::test_support::{
    in_memory_graph_store, DeterministicMemoryEmbedder, TemporaryVectorCandidateStore,
};
use crate::{CharacterMemory, CustomError};

const SUBJECT: MemoryId = MemoryId::from_u128(1);
const SOURCE: MemoryId = MemoryId::from_u128(2);
const OLD: MemoryId = MemoryId::from_u128(3);
const REPLACEMENT: MemoryId = MemoryId::from_u128(4);
const NEW: MemoryId = MemoryId::from_u128(5);

// The gates only inject a suspension. Every storage operation reaches a real adapter.
#[derive(Default)]
struct Gate {
    armed: AtomicBool,
    entered: Notify,
    release: Notify,
}

impl Gate {
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    async fn stop_once(&self) {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
    }
}

async fn completes<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .expect("operation must complete without deadlock")
}

async fn enter<F: Future + Unpin>(gate: &Gate, future: &mut F)
where
    F::Output: std::fmt::Debug,
{
    completes(async {
        tokio::select! {
            _ = gate.entered.notified() => {},
            result = future => panic!("operation completed before reaching the armed gate: {result:?}"),
        }
    })
    .await;
}

async fn waits_for_turn(future: &mut (impl Future + Unpin)) {
    assert!(
        tokio::time::timeout(Duration::from_millis(100), future)
            .await
            .is_err(),
        "another writer must wait for the whole durable turn"
    );
}

struct Fixture {
    memory: CharacterMemory,
    graph: Arc<Gate>,
    vector: Arc<Gate>,
    stats: Arc<Gate>,
    embed: Arc<Gate>,
}

impl Fixture {
    async fn new() -> Self {
        let graph = Arc::new(Gate::default());
        let vector = Arc::new(Gate::default());
        let stats = Arc::new(Gate::default());
        let embed = Arc::new(Gate::default());
        let mut memory = CharacterMemory::from_parts(
            Box::new(GatedGraph {
                store: in_memory_graph_store(),
                gate: graph.clone(),
            }),
            Box::new(GatedVector {
                store: TemporaryVectorCandidateStore::open(8).await,
                gate: vector.clone(),
            }),
            Box::new(GatedEmbedder {
                inner: DeterministicMemoryEmbedder::new(8),
                gate: embed.clone(),
            }),
        );
        memory.memory_composition.stats_store = Box::new(GatedStats {
            store: InMemoryRetrievalStatsStore::new(),
            gate: stats.clone(),
        });
        let mut entity = EntityDraft::new();
        entity.id = Some(SUBJECT);
        let mut episode = EpisodeDraft::new("Original source");
        episode.id = Some(SOURCE);
        episode.raw_ref = Some("raw://write-turn/source".to_owned());
        memory
            .remember(
                RememberInput::new("Original source")
                    .with_entity(entity)
                    .with_episode(episode),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        memory
            .commit(
                belief_plan(OLD, "Original belief"),
                CommitOptions::default(),
            )
            .await
            .unwrap();
        Self {
            memory,
            graph,
            vector,
            stats,
            embed,
        }
    }

    async fn belief(&self, id: MemoryId) -> DerivedMemory {
        let objects = self
            .memory
            .memory_composition
            .graph_store
            .query_objects(&GraphObjectQuery::by_ids(vec![id]))
            .await
            .unwrap();
        match objects.as_slice() {
            [MemoryObject::DerivedMemory(memory)] => memory.clone(),
            _ => panic!("expected one persisted belief"),
        }
    }

    async fn vector_ids(&self) -> Vec<MemoryId> {
        let mut ids = self
            .memory
            .memory_composition
            .vector_store
            .search_candidates(&VectorCandidateSearch::new(
                vec![1.0; 8],
                100,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap()
            .candidates
            .iter()
            .map(|item| item.object_id)
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    async fn counter(&self, relation: RelationType) -> RetrievalStatsCounter {
        self.memory
            .memory_composition
            .stats_store
            .counter(&RetrievalStatsCounterKey {
                entity_id: SUBJECT,
                relation_kind: relation,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap()
    }
}

fn belief_plan(id: MemoryId, text: &str) -> RememberWritePlan {
    let mut draft =
        DerivedMemoryDraft::new(DerivedType::Reflection, text).with_source_episode(SOURCE);
    draft.id = Some(id);
    draft.entity_ids = vec![SUBJECT];
    draft.created_at = chrono::DateTime::from_timestamp(1_800_000_000, 123_456_789);
    draft.updated_at = draft.created_at;
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    RememberWritePlan::new()
        .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            draft,
            CandidateProvenance::caller("concurrency fixture"),
        )))
        .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(ObjectType::DerivedMemory, id),
            CandidateProvenance::caller("concurrency fixture"),
        )))
}

fn correction() -> CorrectMemoryDraft {
    let mut replacement =
        ReplacementDerivedMemoryDraft::new(DerivedType::UserPreference, "Corrected preference")
            .with_source_episode(SOURCE);
    replacement.id = Some(REPLACEMENT);
    replacement.correction_origin_provenance = SourceProvenanceReference::episode(SOURCE);
    let mut draft = CorrectMemoryDraft::new(
        CorrectionTarget::derived_memory(OLD),
        "Correction rationale",
    )
    .with_replacement(replacement);
    draft.correction_origin = SourceProvenanceReference::episode(SOURCE);
    draft
}

fn link() -> MemoryLinkDraft {
    MemoryLinkDraft::new(
        ObjectType::DerivedMemory,
        OLD,
        RelationType::AssociatedWith,
        ObjectType::Entity,
        SUBJECT,
    )
}

fn forget(id: MemoryId) -> ForgetMemoryDraft {
    ForgetMemoryDraft::suppress(
        LifecycleTargetRef::derived_memory(id),
        "Suppress the belief",
    )
}

#[tokio::test]
async fn competing_commits_revalidate_the_same_id_before_writing() {
    let fixture = Fixture::new().await;
    fixture.graph.arm();
    let mut first = Box::pin(
        fixture
            .memory
            .commit(belief_plan(NEW, "First writer"), CommitOptions::default()),
    );
    enter(&fixture.graph, &mut first).await;
    let mut second = Box::pin(
        fixture
            .memory
            .commit(belief_plan(NEW, "Second writer"), CommitOptions::default()),
    );
    waits_for_turn(&mut second).await;
    fixture.graph.release.notify_one();
    let (first, second) = completes(async { tokio::join!(first, second) }).await;
    assert_eq!(first.unwrap().vector_indexed_object_ids, vec![NEW]);
    assert!(
        matches!(second, Err(CustomError::DeterministicIdCollision { object }) if object.id == NEW)
    );
    assert_eq!(fixture.belief(NEW).await.text, "First writer");
    assert_eq!(fixture.vector_ids().await, vec![OLD, NEW]);
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 2,
            active_count: 2,
            current_count: 2
        }
    );
}

#[tokio::test]
async fn remember_and_forget_finish_as_commit_then_forget_without_late_vector_or_stats_writes() {
    let fixture = Fixture::new().await;
    let MemoryCandidate::DerivedMemory(candidate) = belief_plan(NEW, "Remember then suppress")
        .candidates
        .remove(0)
    else {
        unreachable!()
    };
    fixture.vector.arm();
    let mut first = Box::pin(fixture.memory.remember(
        RememberInput::new("New source").with_derived_memory(candidate.draft),
        RememberOptions::default(),
    ));
    enter(&fixture.vector, &mut first).await;
    let mut second = Box::pin(fixture.memory.forget(forget(NEW)));
    waits_for_turn(&mut second).await;
    fixture.vector.release.notify_one();
    let (first, second) = completes(async { tokio::join!(first, second) }).await;
    first.unwrap();
    second.unwrap();
    assert_eq!(
        fixture.belief(NEW).await.retention_state,
        RetentionState::Suppressed
    );
    assert_eq!(fixture.vector_ids().await, vec![OLD]);
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 2,
            active_count: 1,
            current_count: 1
        }
    );
}

#[tokio::test]
async fn correction_holds_the_turn_through_stats_before_link_hydrates_endpoints() {
    let fixture = Fixture::new().await;
    fixture.stats.arm();
    let mut first = Box::pin(fixture.memory.correct(correction()));
    enter(&fixture.stats, &mut first).await;
    let mut second = Box::pin(fixture.memory.link(link()));
    waits_for_turn(&mut second).await;
    fixture.stats.release.notify_one();
    let (first, second) = completes(async { tokio::join!(first, second) }).await;
    first.unwrap();
    second.unwrap();
    assert_eq!(fixture.belief(REPLACEMENT).await.supersedes, vec![OLD]);
    assert_eq!(fixture.vector_ids().await, vec![REPLACEMENT]);
    assert_eq!(
        fixture.counter(RelationType::AssociatedWith).await,
        RetrievalStatsCounter {
            total_count: 1,
            active_count: 1,
            current_count: 0
        }
    );
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 2,
            active_count: 2,
            current_count: 1
        }
    );
}

#[tokio::test]
async fn correction_graph_noop_retry_serializes_vector_and_stats_repair_with_forget() {
    let fixture = Fixture::new().await;
    fixture.memory.correct(correction()).await.unwrap();
    fixture.vector.arm();
    let mut retry = Box::pin(fixture.memory.correct(correction()));
    enter(&fixture.vector, &mut retry).await;
    let mut suppression = Box::pin(fixture.memory.forget(forget(REPLACEMENT)));
    waits_for_turn(&mut suppression).await;
    fixture.vector.release.notify_one();
    let (retry, suppression) = completes(async { tokio::join!(retry, suppression) }).await;
    let retry = retry.unwrap();
    assert!(retry.graph_mutated_object_ids.is_empty());
    assert!(retry.graph_mutated_link_ids.is_empty());
    suppression.unwrap();
    assert_eq!(
        fixture.belief(REPLACEMENT).await.retention_state,
        RetentionState::Suppressed
    );
    assert!(fixture.vector_ids().await.is_empty());
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 2,
            active_count: 1,
            current_count: 0
        }
    );
}

#[tokio::test]
async fn stalled_embedding_leaves_other_writes_prepare_and_retrieve_free() {
    for path in ["commit", "remember", "correct", "repair"] {
        let fixture = Fixture::new().await;
        if path == "repair" {
            fixture.memory.correct(correction()).await.unwrap();
        }
        fixture.embed.arm();
        let mut stalled = Box::pin(async {
            match path {
                "commit" => {
                    fixture
                        .memory
                        .commit(belief_plan(NEW, "Stalled commit"), CommitOptions::default())
                        .await
                        .unwrap();
                }
                "remember" => {
                    fixture
                        .memory
                        .remember(
                            RememberInput::new("Stalled remember"),
                            RememberOptions::default(),
                        )
                        .await
                        .unwrap();
                }
                _ => {
                    fixture.memory.correct(correction()).await.unwrap();
                }
            }
        });
        enter(&fixture.embed, &mut stalled).await;
        completes(async {
            fixture
                .memory
                .prepare(
                    RememberInput::new("Preparation stays free"),
                    PrepareOptions::default(),
                )
                .await
                .unwrap();
            fixture
                .memory
                .remember(
                    RememberInput::new("Independent writer"),
                    RememberOptions::default(),
                )
                .await
                .unwrap();
            fixture.memory.link(link()).await.unwrap();
            fixture
                .memory
                .retrieve(RetrievalContext::new("Original source"))
                .await
                .unwrap();
        })
        .await;
        fixture.embed.release.notify_one();
        completes(stalled).await;
    }
}

#[tokio::test]
async fn correction_rebuilds_the_cascade_after_embedding_and_preserves_batch_identity() {
    let fixture = Fixture::new().await;
    let mut draft = correction();
    draft.targets = vec![CorrectionTarget::source_object(
        SourceObjectCorrectionTarget::Episode {
            id: SOURCE,
            original_raw_ref: Some("raw://write-turn/source".to_owned()),
            original_source_ref: None,
        },
    )];
    let mut second =
        ReplacementDerivedMemoryDraft::new(DerivedType::Claim, "A different corrected claim")
            .with_source_episode(SOURCE);
    second.id = Some(MemoryId::from_u128(6));
    second.correction_origin_provenance = SourceProvenanceReference::episode(SOURCE);
    draft.replacement_derived_memories.insert(0, second);
    fixture.embed.arm();
    let mut correction = Box::pin(fixture.memory.correct(draft));
    enter(&fixture.embed, &mut correction).await;
    completes(fixture.memory.commit(
        belief_plan(NEW, "Arrived during embedding"),
        CommitOptions::default(),
    ))
    .await
    .unwrap();
    fixture.embed.release.notify_one();
    completes(correction).await.unwrap();
    assert_eq!(
        fixture.vector_ids().await,
        vec![REPLACEMENT, MemoryId::from_u128(6)]
    );
    for id in [REPLACEMENT, MemoryId::from_u128(6)] {
        let belief = fixture.belief(id).await;
        assert_eq!(belief.supersedes, vec![OLD, NEW]);
        let record =
            crate::policy::memory_object_vector_record(&MemoryObject::DerivedMemory(belief))
                .unwrap();
        let embedding = DeterministicMemoryEmbedder::new(8)
            .embed(&record.embedding_input())
            .await
            .unwrap();
        let recall = fixture
            .memory
            .memory_composition
            .vector_store
            .search_candidates(&VectorCandidateSearch::new(
                embedding,
                1,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap();
        assert_eq!(recall.candidates[0].object_id, id);
        assert!(recall.candidates[0].score > 0.9999);
    }
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 4,
            active_count: 4,
            current_count: 2
        }
    );
}

#[tokio::test]
async fn precomputed_embedding_errors_still_commit_graph_and_stats_then_allow_repair() {
    let mut fixture = Fixture::new().await;
    fixture.memory.memory_composition.embedder = Box::new(FailingEmbedder);
    let plan = belief_plan(NEW, "Persist despite the embedding error");
    let committed = fixture
        .memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        committed.vector_indexing_failure.unwrap().cause,
        crate::errors::VectorIndexingCause::Embedding(crate::errors::EmbeddingError::MissingData)
    ));
    assert_eq!(
        fixture.belief(NEW).await.text,
        "Persist despite the embedding error"
    );
    assert_eq!(fixture.vector_ids().await, vec![OLD]);
    assert_eq!(fixture.counter(RelationType::About).await.current_count, 2);

    let corrected = fixture.memory.correct(correction()).await.unwrap();
    assert!(matches!(
        corrected
            .vector_maintenance_failure
            .unwrap()
            .failures
            .as_slice(),
        [VectorMaintenanceFailureItem {
            operation: VectorMaintenanceOperation::Upsert,
            cause: crate::errors::VectorIndexingCause::Embedding(
                crate::errors::EmbeddingError::MissingData
            ),
            ..
        }]
    ));
    assert_eq!(fixture.belief(REPLACEMENT).await.supersedes, vec![OLD]);
    assert!(fixture.vector_ids().await.is_empty());
    assert_eq!(
        fixture.counter(RelationType::About).await,
        RetrievalStatsCounter {
            total_count: 3,
            active_count: 3,
            current_count: 2
        }
    );

    fixture.memory.memory_composition.embedder = Box::new(DeterministicMemoryEmbedder::new(8));
    let repaired = fixture
        .memory
        .commit(plan, CommitOptions::default())
        .await
        .unwrap();
    assert!(repaired.vector_indexing_failure.is_none());
    let repaired = fixture.memory.correct(correction()).await.unwrap();
    assert!(repaired.graph_mutated_object_ids.is_empty());
    assert!(repaired.vector_maintenance_failure.is_none());
    assert_eq!(fixture.vector_ids().await, vec![REPLACEMENT, NEW]);
}

struct FailingEmbedder;

#[async_trait]
impl MemoryEmbedder for FailingEmbedder {
    async fn embed(&self, _input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        Err(crate::errors::EmbeddingError::MissingData.into())
    }
    async fn embed_batch(&self, _inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        Err(crate::errors::EmbeddingError::MissingData.into())
    }
}

struct GatedGraph {
    store: OxigraphGraphAuthorityStore,
    gate: Arc<Gate>,
}

#[async_trait]
impl GraphAuthorityStore for GatedGraph {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        self.store.upsert_objects(objects).await
    }
    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        self.store.upsert_links(links).await
    }
    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        self.gate.stop_once().await;
        self.store.upsert_objects_and_links(objects, links).await
    }
    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, GraphQueryError> {
        self.store.query_objects(query).await
    }
    async fn query_notions_known_as(&self, name: &str) -> Result<Vec<MemoryId>, GraphQueryError> {
        self.store.query_notions_known_as(name).await
    }
    async fn query_superseded_derived_memory_ids(
        &self,
        ids: &[MemoryId],
    ) -> Result<Vec<MemoryId>, GraphQueryError> {
        self.store.query_superseded_derived_memory_ids(ids).await
    }
    async fn query_links_by_ids(&self, ids: &[MemoryId]) -> Result<Vec<MemoryLink>, CustomError> {
        self.store.query_links_by_ids(ids).await
    }
    async fn query_derived_memories_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        self.store.query_derived_memories_by_provenance(query).await
    }
    async fn query_derived_memories_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        self.store.query_derived_memories_by_thread(query).await
    }
    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        self.store.expand_bounded(query).await
    }
}

struct GatedVector {
    store: TemporaryVectorCandidateStore,
    gate: Arc<Gate>,
}

#[async_trait]
impl VectorCandidateStore for GatedVector {
    async fn close(&self) -> Result<(), CustomError> {
        self.store.close().await
    }
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        self.gate.stop_once().await;
        self.store.upsert_vector_records(records).await
    }
    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        self.store.search_candidates(query).await
    }
    async fn delete_candidates(&self, ids: &[MemoryId]) -> Result<(), CustomError> {
        self.store.delete_candidates(ids).await
    }
}

struct GatedEmbedder {
    inner: DeterministicMemoryEmbedder,
    gate: Arc<Gate>,
}

#[async_trait]
impl MemoryEmbedder for GatedEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        self.inner.embed(input).await
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        self.gate.stop_once().await;
        self.inner.embed_batch(inputs).await
    }
}

struct GatedStats {
    store: InMemoryRetrievalStatsStore,
    gate: Arc<Gate>,
}

#[async_trait]
impl RetrievalStatsStore for GatedStats {
    async fn record_edges(
        &self,
        edges: &[RetrievalStatsEdge],
    ) -> Result<(), RetrievalStatsStoreError> {
        self.store.record_edges(edges).await
    }
    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), RetrievalStatsStoreError> {
        self.gate.stop_once().await;
        self.store.record_object_states(states).await
    }
    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        self.store.counter(key).await
    }
    async fn global_counter(
        &self,
        relation: RelationType,
        object_type: ObjectType,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        self.store.global_counter(relation, object_type).await
    }
    async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
        self.store.health().await
    }
    async fn mark_unhealthy(
        &self,
        cause: RetrievalStatsHealthCause,
    ) -> Result<(), RetrievalStatsStoreError> {
        self.store.mark_unhealthy(cause).await
    }
}
