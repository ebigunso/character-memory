#![allow(dead_code)]

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::api::types::{
    DerivedMemory, DerivedType, Entity, EntityType, Episode, MemoryId, MemoryLink, MemoryObject,
    MemoryThread, Modality, ObjectType, Observation, RelationType, RetentionState, Stability,
    ThreadStatus, DEFAULT_SCHEMA_VERSION,
};
use crate::errors::CustomError;
use crate::internal::models::vector::{
    EmbeddingInput, VectorCandidateMatch, VectorCandidateRecord, VectorCandidateSearch,
    VectorSurface,
};
use crate::internal::repositories::{
    GraphAuthorityStore, GraphExpansion, GraphExpansionQuery, GraphObjectQuery, MemoryEmbedder,
    RawReference, RawReferenceResolver, VectorCandidateStore,
};

#[derive(Debug, Default)]
pub(crate) struct FakeVectorCandidateStore {
    records: Mutex<Vec<VectorCandidateRecord>>,
}

impl FakeVectorCandidateStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl VectorCandidateStore for FakeVectorCandidateStore {
    async fn upsert_candidates(
        &self,
        candidates: &[VectorCandidateRecord],
    ) -> Result<(), CustomError> {
        let mut records = lock(&self.records)?;

        for candidate in candidates {
            records.retain(|record| {
                record.object_id != candidate.object_id || record.surface != candidate.surface
            });
            records.push(candidate.clone());
        }

        Ok(())
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        let records = lock(&self.records)?;
        let mut matches: Vec<_> = records
            .iter()
            .filter(|record| {
                query.object_types.is_empty() || query.object_types.contains(&record.object_type)
            })
            .map(|record| {
                VectorCandidateMatch::new(
                    record.object_id,
                    record.object_type,
                    record.surface,
                    cosine_similarity(&query.query_embedding, &record.embedding),
                )
            })
            .collect();

        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.object_id.cmp(&right.object_id))
                .then_with(|| {
                    object_type_rank(left.object_type).cmp(&object_type_rank(right.object_type))
                })
                .then_with(|| surface_rank(left.surface).cmp(&surface_rank(right.surface)))
        });
        matches.truncate(query.limit);

        Ok(matches)
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        let delete_ids: HashSet<_> = object_ids.iter().copied().collect();
        lock(&self.records)?.retain(|record| !delete_ids.contains(&record.object_id));
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct FakeGraphAuthorityStore {
    objects: Mutex<Vec<MemoryObject>>,
    links: Mutex<Vec<MemoryLink>>,
}

impl FakeGraphAuthorityStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GraphAuthorityStore for FakeGraphAuthorityStore {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        let mut stored = lock(&self.objects)?;

        for object in objects {
            let (object_id, object_type) = object_identity(object);
            stored.retain(|existing| object_identity(existing) != (object_id, object_type));
            stored.push(object.clone());
        }

        Ok(())
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        let mut stored = lock(&self.links)?;

        for link in links {
            stored.retain(|existing| existing.id != link.id);
            stored.push(link.clone());
        }

        Ok(())
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        let mut objects: Vec<_> = lock(&self.objects)?
            .iter()
            .filter(|object| {
                let (object_id, object_type) = object_identity(object);
                (query.object_ids.is_empty() || query.object_ids.contains(&object_id))
                    && (query.object_types.is_empty() || query.object_types.contains(&object_type))
            })
            .cloned()
            .collect();

        sort_objects(&mut objects);
        if let Some(limit) = query.limit {
            objects.truncate(limit);
        }

        Ok(objects)
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        if query.max_nodes == 0 {
            return Ok(GraphExpansion::new(Vec::new(), Vec::new()));
        }

        let objects = lock(&self.objects)?.clone();
        let links = lock(&self.links)?.clone();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([(query.root_id, query.root_type, 0_u8)]);

        while let Some((object_id, object_type, depth)) = queue.pop_front() {
            if visited.len() >= query.max_nodes || !visited.insert((object_id, object_type)) {
                continue;
            }

            if depth >= query.max_depth {
                continue;
            }

            let mut neighbors: Vec<_> = links
                .iter()
                .filter_map(|link| {
                    if link.from_id == object_id && link.from_type == object_type {
                        Some((link.to_id, link.to_type))
                    } else if link.to_id == object_id && link.to_type == object_type {
                        Some((link.from_id, link.from_type))
                    } else {
                        None
                    }
                })
                .filter(|(_, neighbor_type)| {
                    query.allowed_object_types.is_empty()
                        || query.allowed_object_types.contains(neighbor_type)
                })
                .collect();
            neighbors.sort_by_key(|node| stable_node_key(*node));

            for neighbor in neighbors {
                if visited.len() + queue.len() >= query.max_nodes && !visited.contains(&neighbor) {
                    continue;
                }
                queue.push_back((neighbor.0, neighbor.1, depth + 1));
            }
        }

        let mut expanded_objects: Vec<_> = objects
            .into_iter()
            .filter(|object| visited.contains(&object_identity(object)))
            .collect();
        sort_objects(&mut expanded_objects);

        let mut expanded_links: Vec<_> = links
            .into_iter()
            .filter(|link| {
                visited.contains(&(link.from_id, link.from_type))
                    && visited.contains(&(link.to_id, link.to_type))
            })
            .collect();
        expanded_links.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(GraphExpansion::new(expanded_objects, expanded_links))
    }
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

#[derive(Debug, Default)]
pub(crate) struct FixtureRawReferenceResolver {
    entries: Mutex<Vec<RawReference>>,
}

impl FixtureRawReferenceResolver {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&self, reference: impl Into<String>, text: impl Into<String>) {
        let raw_reference = RawReference::new(reference, text);
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|entry| entry.reference != raw_reference.reference);
        entries.push(raw_reference);
    }

    pub(crate) fn insert_file(
        &self,
        reference: impl Into<String>,
        path: &Path,
    ) -> Result<(), CustomError> {
        let text = fs::read_to_string(path)
            .map_err(|error| CustomError::DatabaseError(error.to_string()))?;
        self.insert(reference, text);
        Ok(())
    }
}

#[async_trait]
impl RawReferenceResolver for FixtureRawReferenceResolver {
    async fn resolve(&self, reference: &str) -> Result<Option<RawReference>, CustomError> {
        Ok(lock(&self.entries)?
            .iter()
            .find(|entry| entry.reference == reference)
            .cloned())
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
    let user_entity = entity(
        fixture_id(1),
        EntityType::User,
        "Kohta",
        Some("person:kohta"),
    );
    let assistant_entity = entity(
        fixture_id(2),
        EntityType::Assistant,
        "Assistant",
        Some("assistant:default"),
    );
    let project_entity = entity(
        fixture_id(3),
        EntityType::Project,
        "CharacterMemory",
        Some("project:character-memory"),
    );
    let hub_entity = entity(
        fixture_id(4),
        EntityType::Concept,
        "v0.1 contracts",
        Some("concept:v0.1-contracts"),
    );
    let soft_thread = soft_thread();
    let derived_reflection = derived_memory(
        fixture_id(30),
        DerivedType::Reflection,
        "The user wants service-free contract tests before pipeline wiring.",
        episode.id,
        salient_observation.id,
        vec![soft_thread.id],
        vec![user_entity.id, project_entity.id],
        true,
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
        true,
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
        true,
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
        true,
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
        true,
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
        false,
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

pub(crate) fn simple_episode() -> Episode {
    Episode {
        id: fixture_id(10),
        object_type: ObjectType::Episode,
        modality: Modality::Chat,
        source_conversation_id: Some("conversation:v0.1-fixture".to_owned()),
        started_at: Some(timestamp("2026-04-27T10:00:00Z")),
        ended_at: Some(timestamp("2026-04-27T10:10:00Z")),
        participant_entity_ids: vec![fixture_id(1), fixture_id(2)],
        summary: "Discussed deterministic v0.1 store contract fixtures.".to_owned(),
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

fn entity(
    id: MemoryId,
    entity_type: EntityType,
    name: impl Into<String>,
    canonical_key: Option<&str>,
) -> Entity {
    Entity {
        id,
        object_type: ObjectType::Entity,
        entity_type,
        name: name.into(),
        aliases: Vec::new(),
        canonical_key: canonical_key.map(str::to_owned),
        summary: Some("Representative fixture entity.".to_owned()),
        created_at: timestamp("2026-04-27T10:11:02Z"),
        updated_at: timestamp("2026-04-27T10:11:03Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn soft_thread() -> MemoryThread {
    MemoryThread {
        id: fixture_id(25),
        object_type: ObjectType::MemoryThread,
        title: "v0.1 contract test support".to_owned(),
        summary: "Soft thread connecting store-contract fixture objects.".to_owned(),
        status: ThreadStatus::Active,
        last_touched_at: timestamp("2026-04-27T10:11:04Z"),
        salience_score: 0.7,
        canonical_key: Some("thread:v0.1-contract-test-support".to_owned()),
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
    is_current: bool,
    supersedes: Vec<MemoryId>,
    retention_state: RetentionState,
) -> DerivedMemory {
    DerivedMemory {
        id,
        object_type: ObjectType::DerivedMemory,
        derived_type,
        text: text.into(),
        derived_from_episode_ids: vec![episode_id],
        derived_from_observation_ids: vec![observation_id],
        thread_ids,
        entity_ids,
        confidence: 0.85,
        salience_score: 0.75,
        stability: Stability::Medium,
        is_current,
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
        confidence: 0.9,
        rationale: Some("Representative fixture link.".to_owned()),
        created_at: timestamp("2026-04-27T10:11:07Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
    mutex
        .lock()
        .map_err(|error| CustomError::DatabaseError(format!("test support lock poisoned: {error}")))
}

fn fixture_id(suffix: u128) -> MemoryId {
    Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0000 + suffix)
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

fn sort_objects(objects: &mut [MemoryObject]) {
    objects.sort_by(|left, right| {
        stable_node_key(object_identity(left)).cmp(&stable_node_key(object_identity(right)))
    });
}

fn stable_node_key(node: (MemoryId, ObjectType)) -> (MemoryId, u8) {
    (node.0, object_type_rank(node.1))
}

fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

fn surface_rank(surface: VectorSurface) -> u8 {
    match surface {
        VectorSurface::Summary => 0,
        VectorSurface::Text => 1,
        VectorSurface::Name => 2,
        VectorSurface::DerivedText => 3,
        VectorSurface::Query => 4,
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot_product: f32 = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum();
    let left_magnitude = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_magnitude = right.iter().map(|value| value * value).sum::<f32>().sqrt();

    if left_magnitude == 0.0 || right_magnitude == 0.0 {
        0.0
    } else {
        dot_product / (left_magnitude * right_magnitude)
    }
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
    async fn vector_fake_upserts_searches_and_deletes_deterministically() {
        let store = FakeVectorCandidateStore::new();
        let fixtures = representative_fixtures();

        store
            .upsert_candidates(&[
                VectorCandidateRecord::new(
                    fixtures.episode.id,
                    ObjectType::Episode,
                    VectorSurface::Summary,
                    vec![1.0, 0.0],
                ),
                VectorCandidateRecord::new(
                    fixtures.salient_observation.id,
                    ObjectType::Observation,
                    VectorSurface::Text,
                    vec![0.0, 1.0],
                ),
            ])
            .await
            .unwrap();

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10);
        let first_result = store.search_candidates(&query).await.unwrap();
        let second_result = store.search_candidates(&query).await.unwrap();

        assert_eq!(first_result, second_result);
        assert_eq!(first_result[0].object_id, fixtures.episode.id);
        assert_eq!(first_result[0].object_type, ObjectType::Episode);
        assert_eq!(first_result[0].surface, VectorSurface::Summary);

        store
            .delete_candidates(&[fixtures.episode.id])
            .await
            .unwrap();
        let after_delete = store.search_candidates(&query).await.unwrap();

        assert_eq!(after_delete.len(), 1);
        assert_eq!(after_delete[0].object_id, fixtures.salient_observation.id);
    }

    #[tokio::test]
    async fn graph_fake_preserves_objects_links_lifecycle_and_raw_refs() {
        let store = FakeGraphAuthorityStore::new();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let queried = store
            .query_objects(&GraphObjectQuery::by_ids(vec![
                fixtures.episode.id,
                fixtures.suppressed_seed.id,
            ]))
            .await
            .unwrap();

        assert!(queried.contains(&MemoryObject::Episode(fixtures.episode.clone())));
        assert!(queried.contains(&MemoryObject::DerivedMemory(
            fixtures.suppressed_seed.clone()
        )));
        assert_eq!(
            fixtures.episode.raw_ref.as_deref(),
            Some("file:fixtures/raw/simple-episode.txt")
        );
        assert_eq!(
            fixtures.suppressed_seed.retention_state,
            RetentionState::Suppressed
        );
        assert!(!fixtures.suppressed_seed.is_current);

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.hub_entity.id,
                ObjectType::Entity,
                1,
                5,
            ))
            .await
            .unwrap();

        assert!(expansion
            .objects
            .contains(&MemoryObject::Entity(fixtures.hub_entity.clone())));
        assert!(expansion
            .objects
            .contains(&MemoryObject::Episode(fixtures.episode.clone())));
        assert!(expansion.links.contains(&fixtures.hub_links[0]));
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

    #[tokio::test]
    async fn raw_reference_resolver_uses_fixture_backed_file_content() {
        let resolver = FixtureRawReferenceResolver::new();
        let path = std::env::temp_dir().join(format!("cmem-raw-ref-{}.txt", Uuid::new_v4()));
        fs::write(&path, "raw fixture text").unwrap();

        resolver
            .insert_file("file:fixture/raw-reference.txt", &path)
            .unwrap();
        let resolved = resolver
            .resolve("file:fixture/raw-reference.txt")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resolved.reference, "file:fixture/raw-reference.txt");
        assert_eq!(resolved.text, "raw fixture text");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn representative_fixtures_cover_v0_1_scenarios() {
        let fixtures = representative_fixtures();

        assert_eq!(fixtures.episode.object_type, ObjectType::Episode);
        assert_eq!(
            fixtures.salient_observation.object_type,
            ObjectType::Observation
        );
        assert_eq!(fixtures.user_entity.entity_type, EntityType::User);
        assert_eq!(fixtures.soft_thread.object_type, ObjectType::MemoryThread);
        assert_eq!(
            fixtures.derived_reflection.derived_type,
            DerivedType::Reflection
        );
        assert_eq!(
            fixtures.user_preference.derived_type,
            DerivedType::UserPreference
        );
        assert_eq!(fixtures.open_loop.derived_type, DerivedType::OpenLoop);
        assert_eq!(fixtures.commitment.derived_type, DerivedType::Commitment);
        assert_eq!(fixtures.correction.derived_type, DerivedType::Correction);
        assert_eq!(
            fixtures.suppressed_seed.retention_state,
            RetentionState::Suppressed
        );
        assert_eq!(
            fixtures.soft_thread_link.relation,
            RelationType::PartOfThread
        );
        assert!(fixtures
            .hub_links
            .iter()
            .any(|link| link.from_id == fixtures.hub_entity.id));
    }
}
