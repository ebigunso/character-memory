// Deterministic test harness shared by pipeline, adapter, and facade tests.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
use crate::adapters::qdrant_edge::QdrantEdgeVectorCandidateStore;
use crate::domain::{
    DerivedMemory, DerivedType, Entity, Episode, MemoryId, MemoryLink, MemoryObject,
    MemoryObjectRef, MemoryThread, Modality, ObjectType, Observation, RelationType, RetentionState,
    ThreadStatus, DEFAULT_SCHEMA_VERSION,
};
use crate::errors::CustomError;
use crate::models::vector::{
    EmbeddingInput, VectorCandidateSearch, VectorRecordEmbedding, VectorSurface,
};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};

#[derive(Debug)]
pub(crate) struct TemporaryVectorCandidateStore {
    store: Option<QdrantEdgeVectorCandidateStore>,
    directory: tempfile::TempDir,
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
        }
    }

    fn store(&self) -> &QdrantEdgeVectorCandidateStore {
        self.store.as_ref().expect("temporary vector store is open")
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
        self.store().upsert_vector_records(records).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        self.store().search_candidates(query).await
    }

    async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
        self.store().delete_candidates(objects).await
    }
}

pub(crate) fn in_memory_graph_store() -> OxigraphGraphAuthorityStore {
    OxigraphGraphAuthorityStore::new_in_memory().expect("in-memory graph store")
}

#[derive(Debug, Clone)]
pub(crate) struct DeterministicMemoryEmbedder {
    dimensions: usize,
}

impl DeterministicMemoryEmbedder {
    pub(crate) fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

#[async_trait]
impl MemoryEmbedder for DeterministicMemoryEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        Ok(deterministic_embedding(input, self.dimensions))
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let embeddings = inputs
            .iter()
            .map(|input| deterministic_embedding(input, self.dimensions))
            .collect();

        Ok(embeddings)
    }
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
        source_conversation_id: Some("conversation:contract-fixture".to_owned()),
        started_at: Some(timestamp("2026-04-27T10:00:00Z")),
        ended_at: Some(timestamp("2026-04-27T10:10:00Z")),
        participant_entity_ids: vec![fixture_id(1), fixture_id(2)],
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
        assertions: Vec::new(),
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

fn timestamp(value: &str) -> DateTime<Utc> {
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

    #[tokio::test]
    async fn deterministic_embedder_uses_explicit_text_without_external_services() {
        let embedder = DeterministicMemoryEmbedder::new(8);
        let input = EmbeddingInput::new(
            Some(fixture_id(20)),
            Some(ObjectType::Observation),
            VectorSurface::Text,
            "service-free deterministic embedding",
        );

        let first = embedder.embed(&input).await.unwrap();
        let second = embedder.embed(&input).await.unwrap();
        let different = embedder
            .embed(&EmbeddingInput::new(
                Some(fixture_id(20)),
                Some(ObjectType::Observation),
                VectorSurface::Text,
                "different text",
            ))
            .await
            .unwrap();

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert_eq!(first.len(), 8);
    }
}
