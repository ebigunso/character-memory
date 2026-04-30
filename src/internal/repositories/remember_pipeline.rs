// Remember pipeline used by the public facade and internal tests. Some
// builders remain available for focused test and validation paths.
use crate::api::types::{
    DraftDefaults, MemoryId, MemoryLink, MemoryLinkDraft, MemoryObject, MemoryObjectDraft,
};
use crate::errors::CustomError;
use crate::internal::models::vector::{
    memory_object_vector_record, VectorRecord, VectorRecordEmbedding,
};
use crate::internal::repositories::{GraphAuthorityStore, MemoryEmbedder, VectorCandidateStore};

#[derive(Debug, Clone)]
pub(crate) struct RememberPipelineDraft {
    object_drafts: Vec<MemoryObjectDraft>,
    link_drafts: Vec<MemoryLinkDraft>,
    defaults: DraftDefaults,
}

impl RememberPipelineDraft {
    pub(crate) fn new(
        object_drafts: impl IntoIterator<Item = MemoryObjectDraft>,
        link_drafts: impl IntoIterator<Item = MemoryLinkDraft>,
    ) -> Self {
        Self {
            object_drafts: object_drafts.into_iter().collect(),
            link_drafts: link_drafts.into_iter().collect(),
            defaults: DraftDefaults::generated(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_defaults(mut self, defaults: DraftDefaults) -> Self {
        self.defaults = defaults;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RememberPipelineOutcome {
    pub(crate) persisted_object_ids: Vec<MemoryId>,
    pub(crate) persisted_link_ids: Vec<MemoryId>,
    pub(crate) vector_indexed_object_ids: Vec<MemoryId>,
    pub(crate) vector_indexing_failure: Option<VectorIndexingFailure>,
}

impl RememberPipelineOutcome {
    fn graph_persisted(objects: &[MemoryObject], links: &[MemoryLink]) -> Self {
        Self {
            persisted_object_ids: objects.iter().map(memory_object_id).collect(),
            persisted_link_ids: links.iter().map(|link| link.id).collect(),
            vector_indexed_object_ids: Vec::new(),
            vector_indexing_failure: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VectorIndexingFailure {
    pub(crate) unindexed_object_ids: Vec<MemoryId>,
    pub(crate) error_message: String,
}

pub(crate) struct RememberPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    graph_store: &'a G,
    vector_store: &'a V,
    embedder: &'a E,
}

impl<'a, G, V, E> RememberPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    pub(crate) fn new(graph_store: &'a G, vector_store: &'a V, embedder: &'a E) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
        }
    }

    pub(crate) async fn remember(
        &self,
        draft: RememberPipelineDraft,
    ) -> Result<RememberPipelineOutcome, CustomError> {
        let (objects, links) = validated_domain_values(draft)?;
        let vector_records = vector_records_for_objects(&objects);

        self.graph_store.upsert_objects(&objects).await?;
        self.graph_store.upsert_links(&links).await?;

        let mut outcome = RememberPipelineOutcome::graph_persisted(&objects, &links);
        if vector_records.is_empty() {
            return Ok(outcome);
        }

        match self.index_vector_records(&vector_records).await {
            Ok(indexed_ids) => {
                outcome.vector_indexed_object_ids = indexed_ids;
            }
            Err(error_message) => {
                outcome.vector_indexing_failure = Some(VectorIndexingFailure {
                    unindexed_object_ids: vector_records
                        .iter()
                        .map(|record| record.object_id)
                        .collect(),
                    error_message,
                });
            }
        }

        Ok(outcome)
    }

    async fn index_vector_records(
        &self,
        vector_records: &[VectorRecord],
    ) -> Result<Vec<MemoryId>, String> {
        let embedding_inputs = vector_records
            .iter()
            .map(VectorRecord::embedding_input)
            .collect::<Vec<_>>();
        let embeddings = self
            .embedder
            .embed_batch(&embedding_inputs)
            .await
            .map_err(|error| error.to_string())?;

        if embeddings.len() != vector_records.len() {
            return Err(format!(
                "embedder returned {} embeddings for {} vector records",
                embeddings.len(),
                vector_records.len()
            ));
        }

        let record_embeddings = vector_records
            .iter()
            .zip(embeddings.iter())
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding.as_slice()))
            .collect::<Vec<_>>();
        self.vector_store
            .upsert_vector_records(&record_embeddings)
            .await
            .map_err(|error| error.to_string())?;

        Ok(vector_records
            .iter()
            .map(|record| record.object_id)
            .collect())
    }
}

fn validated_domain_values(
    draft: RememberPipelineDraft,
) -> Result<(Vec<MemoryObject>, Vec<MemoryLink>), CustomError> {
    let mut defaults = draft.defaults;
    let mut objects = Vec::new();
    let mut links = Vec::new();

    for draft in draft.object_drafts {
        match draft
            .into_domain_with_defaults(&mut defaults)
            .map_err(validation_error)?
        {
            MemoryObject::MemoryLink(link) => links.push(link),
            object => objects.push(object),
        }
    }

    for draft in draft.link_drafts {
        links.push(
            draft
                .into_domain_with_defaults(&mut defaults)
                .map_err(validation_error)?,
        );
    }

    Ok((objects, links))
}

fn vector_records_for_objects(objects: &[MemoryObject]) -> Vec<VectorRecord> {
    objects
        .iter()
        .filter_map(memory_object_vector_record)
        .collect()
}

fn validation_error(error: impl ToString) -> CustomError {
    CustomError::MemoryValidation(error.to_string())
}

fn memory_object_id(object: &MemoryObject) -> MemoryId {
    match object {
        MemoryObject::Episode(object) => object.id,
        MemoryObject::Observation(object) => object.id,
        MemoryObject::Entity(object) => object.id,
        MemoryObject::MemoryThread(object) => object.id,
        MemoryObject::DerivedMemory(object) => object.id,
        MemoryObject::MemoryLink(object) => object.id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex, MutexGuard};
    use uuid::Uuid;

    use crate::api::types::{
        DerivedMemoryDraft, DerivedType, EntityDraft, EntityType, EpisodeDraft, MemoryThreadDraft,
        ObjectType, ObservationDraft, RelationType,
    };
    use crate::internal::models::vector::{
        EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
    };
    use crate::internal::repositories::{GraphExpansion, GraphExpansionQuery, GraphObjectQuery};

    #[tokio::test]
    async fn persists_graph_objects_links_then_vectors_in_stable_order() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .remember(representative_draft(&ids))
            .await
            .expect("remember draft should persist");

        assert_eq!(
            outcome.persisted_object_ids,
            vec![
                ids.entity,
                ids.episode,
                ids.observation,
                ids.thread,
                ids.derived
            ]
        );
        assert_eq!(
            outcome.persisted_link_ids,
            vec![ids.inline_link, ids.extra_link]
        );
        assert_eq!(
            outcome.vector_indexed_object_ids,
            outcome.persisted_object_ids
        );
        assert_eq!(outcome.vector_indexing_failure, None);
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphObjects(vec![
                    ids.entity,
                    ids.episode,
                    ids.observation,
                    ids.thread,
                    ids.derived,
                ]),
                StoreCall::GraphLinks(vec![ids.inline_link, ids.extra_link]),
            ]
        );
        assert_eq!(
            embedder.calls(),
            vec![StoreCall::EmbedBatch(vec![
                ids.entity,
                ids.episode,
                ids.observation,
                ids.thread,
                ids.derived,
            ])]
        );
        assert_eq!(
            vector.calls(),
            vec![StoreCall::VectorUpsert(vec![
                ids.entity,
                ids.episode,
                ids.observation,
                ids.thread,
                ids.derived,
            ])]
        );
    }

    #[tokio::test]
    async fn graph_object_failure_prevents_link_embedding_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_objects();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .remember(representative_draft(&ids))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("object write failed"));
        assert_eq!(
            graph.calls(),
            vec![StoreCall::GraphObjects(vec![
                ids.entity,
                ids.episode,
                ids.observation,
                ids.thread,
                ids.derived,
            ])]
        );
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_link_failure_prevents_embedding_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_links();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .remember(representative_draft(&ids))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("link write failed"));
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphObjects(vec![
                    ids.entity,
                    ids.episode,
                    ids.observation,
                    ids.thread,
                    ids.derived,
                ]),
                StoreCall::GraphLinks(vec![ids.inline_link, ids.extra_link]),
            ]
        );
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn validation_failure_prevents_all_store_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let mut invalid_episode = EpisodeDraft::new(" ");
        invalid_episode.id = Some(ids.episode);
        let draft = RememberPipelineDraft::new(
            [MemoryObjectDraft::Episode(invalid_episode)],
            Vec::<MemoryLinkDraft>::new(),
        );
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline.remember(draft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("episode summary must not be empty"));
        assert!(graph.calls().is_empty());
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn memory_link_objects_are_persisted_as_links_and_skipped_for_vectors() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let link_only_draft = RememberPipelineDraft::new(
            [MemoryObjectDraft::MemoryLink(link_draft(
                ids.inline_link,
                ids.episode,
                ids.observation,
            ))],
            Vec::<MemoryLinkDraft>::new(),
        );
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .remember(link_only_draft)
            .await
            .expect("link-only remember draft should persist graph link");

        assert!(outcome.persisted_object_ids.is_empty());
        assert_eq!(outcome.persisted_link_ids, vec![ids.inline_link]);
        assert!(outcome.vector_indexed_object_ids.is_empty());
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn vector_upsert_failure_returns_partial_success_with_graph_ids() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default().fail_upsert();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .remember(representative_draft(&ids))
            .await
            .expect("graph success with vector failure should return partial outcome");

        assert_eq!(
            outcome.persisted_object_ids,
            vec![
                ids.entity,
                ids.episode,
                ids.observation,
                ids.thread,
                ids.derived
            ]
        );
        assert_eq!(
            outcome.persisted_link_ids,
            vec![ids.inline_link, ids.extra_link]
        );
        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("vector failure should be explicit");
        assert_eq!(failure.unindexed_object_ids, outcome.persisted_object_ids);
        assert!(failure.error_message.contains("vector write failed"));
    }

    #[tokio::test]
    async fn wrong_embedding_count_returns_clear_partial_failure_without_vector_write() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default().with_embedding_count(4);
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .remember(representative_draft(&ids))
            .await
            .expect("graph success with embedding mismatch should return partial outcome");

        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("embedding mismatch should be explicit");
        assert_eq!(failure.unindexed_object_ids, outcome.persisted_object_ids);
        assert_eq!(
            failure.error_message,
            "embedder returned 4 embeddings for 5 vector records"
        );
        assert!(vector.calls().is_empty());
    }

    #[derive(Debug, Clone, Copy)]
    struct FixedIds {
        entity: MemoryId,
        episode: MemoryId,
        observation: MemoryId,
        thread: MemoryId,
        derived: MemoryId,
        inline_link: MemoryId,
        extra_link: MemoryId,
    }

    fn fixed_ids() -> FixedIds {
        FixedIds {
            entity: id("550e8400-e29b-41d4-a716-446655443001"),
            episode: id("550e8400-e29b-41d4-a716-446655443002"),
            observation: id("550e8400-e29b-41d4-a716-446655443003"),
            thread: id("550e8400-e29b-41d4-a716-446655443004"),
            derived: id("550e8400-e29b-41d4-a716-446655443005"),
            inline_link: id("550e8400-e29b-41d4-a716-446655443006"),
            extra_link: id("550e8400-e29b-41d4-a716-446655443007"),
        }
    }

    fn representative_draft(ids: &FixedIds) -> RememberPipelineDraft {
        RememberPipelineDraft::new(
            [
                MemoryObjectDraft::Entity(entity_draft(ids.entity)),
                MemoryObjectDraft::Episode(episode_draft(ids.episode)),
                MemoryObjectDraft::Observation(observation_draft(ids.observation, ids.episode)),
                MemoryObjectDraft::MemoryThread(thread_draft(ids.thread)),
                MemoryObjectDraft::DerivedMemory(derived_draft(
                    ids.derived,
                    ids.episode,
                    ids.observation,
                )),
                MemoryObjectDraft::MemoryLink(link_draft(
                    ids.inline_link,
                    ids.episode,
                    ids.observation,
                )),
            ],
            [typed_link_draft(
                ids.extra_link,
                ObjectType::DerivedMemory,
                ids.derived,
                RelationType::PartOfThread,
                ObjectType::MemoryThread,
                ids.thread,
            )],
        )
        .with_defaults(DraftDefaults::at(timestamp()))
    }

    fn entity_draft(id: MemoryId) -> EntityDraft {
        let mut draft = EntityDraft::new(EntityType::User, "Kohta");
        draft.id = Some(id);
        draft
    }

    fn episode_draft(id: MemoryId) -> EpisodeDraft {
        let mut draft = EpisodeDraft::new("Discussed stable remember ordering.");
        draft.id = Some(id);
        draft
    }

    fn observation_draft(id: MemoryId, episode_id: MemoryId) -> ObservationDraft {
        let mut draft = ObservationDraft::new(episode_id, "Stable vectors follow graph writes.");
        draft.id = Some(id);
        draft
    }

    fn thread_draft(id: MemoryId) -> MemoryThreadDraft {
        let mut draft = MemoryThreadDraft::new("Remember pipeline", "Graph before vectors.");
        draft.id = Some(id);
        draft
    }

    fn derived_draft(
        id: MemoryId,
        episode_id: MemoryId,
        observation_id: MemoryId,
    ) -> DerivedMemoryDraft {
        let mut draft = DerivedMemoryDraft::new(DerivedType::Reflection, "Order matters.")
            .with_source_episode(episode_id)
            .with_source_observation(observation_id);
        draft.id = Some(id);
        draft
    }

    fn link_draft(id: MemoryId, from_id: MemoryId, to_id: MemoryId) -> MemoryLinkDraft {
        typed_link_draft(
            id,
            ObjectType::Episode,
            from_id,
            RelationType::Mentions,
            ObjectType::Observation,
            to_id,
        )
    }

    fn typed_link_draft(
        id: MemoryId,
        from_type: ObjectType,
        from_id: MemoryId,
        relation: RelationType,
        to_type: ObjectType,
        to_id: MemoryId,
    ) -> MemoryLinkDraft {
        let mut draft = MemoryLinkDraft::new(from_type, from_id, relation, to_type, to_id);
        draft.id = Some(id);
        draft
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StoreCall {
        GraphObjects(Vec<MemoryId>),
        GraphLinks(Vec<MemoryId>),
        EmbedBatch(Vec<MemoryId>),
        VectorUpsert(Vec<MemoryId>),
    }

    #[derive(Debug, Default)]
    struct RecordingGraphStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_objects: bool,
        fail_links: bool,
    }

    impl RecordingGraphStore {
        fn fail_objects(mut self) -> Self {
            self.fail_objects = true;
            self
        }

        fn fail_links(mut self) -> Self {
            self.fail_links = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for RecordingGraphStore {
        async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(memory_object_id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            Ok(())
        }

        async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphLinks(
                links.iter().map(|link| link.id).collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            Ok(())
        }

        async fn upsert_objects_and_links(
            &self,
            objects: &[MemoryObject],
            links: &[MemoryLink],
        ) -> Result<(), CustomError> {
            self.upsert_objects(objects).await?;
            self.upsert_links(links).await
        }

        async fn query_objects(
            &self,
            _query: &GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &crate::internal::repositories::GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<crate::api::types::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            _query: &crate::internal::repositories::GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<crate::api::types::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Ok(GraphExpansion::new(Vec::new(), Vec::new()))
        }
    }

    #[derive(Debug, Default)]
    struct RecordingVectorStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_upsert: bool,
    }

    impl RecordingVectorStore {
        fn fail_upsert(mut self) -> Self {
            self.fail_upsert = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl VectorCandidateStore for RecordingVectorStore {
        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::VectorUpsert(
                records
                    .iter()
                    .map(|record| record.record.object_id)
                    .collect(),
            ));
            if self.fail_upsert {
                return Err(CustomError::DatabaseError("vector write failed".to_owned()));
            }
            Ok(())
        }

        async fn search_candidates(
            &self,
            _query: &VectorCandidateSearch,
        ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingEmbedder {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        embedding_count: Option<usize>,
    }

    impl RecordingEmbedder {
        fn with_embedding_count(mut self, embedding_count: usize) -> Self {
            self.embedding_count = Some(embedding_count);
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl MemoryEmbedder for RecordingEmbedder {
        async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(vec![embedding_seed(input)])
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            lock(&self.calls).push(StoreCall::EmbedBatch(
                inputs.iter().filter_map(|input| input.object_id).collect(),
            ));
            let count = self.embedding_count.unwrap_or(inputs.len());
            Ok(inputs
                .iter()
                .cycle()
                .take(count)
                .map(|input| vec![embedding_seed(input)])
                .collect())
        }
    }

    fn embedding_seed(input: &EmbeddingInput) -> f32 {
        input.text.len() as f32
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }
}
