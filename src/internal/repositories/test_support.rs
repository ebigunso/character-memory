// Transitional test harness: exposes reusable fakes and fixtures for
// downstream adapter/pipeline tests before every helper is used. Remove once
// downstream tests consume the full support surface, or prune unused helpers.
#![allow(dead_code)]

use std::collections::HashSet;
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
    EmbeddingInput, VectorCandidateFilters, VectorCandidateMatch, VectorCandidateRecord,
    VectorCandidateSearch, VectorRecordEmbedding, VectorSurface, VectorTimeField,
    VectorTimeRangeFilter,
};
use crate::internal::repositories::{
    bounded_expansion, GraphAuthorityStore, GraphExpansion, GraphExpansionQuery, GraphObjectQuery,
    MemoryEmbedder, RawReference, RawReferenceResolver, VectorCandidateStore,
};

#[derive(Debug, Default)]
pub(crate) struct FakeVectorCandidateStore {
    records: Mutex<Vec<VectorCandidateRecord>>,
}

impl FakeVectorCandidateStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn upsert_candidates(
        &self,
        candidates: &[VectorCandidateRecord],
    ) -> Result<(), CustomError> {
        self.replace_candidates(candidates)
    }

    fn replace_candidates(&self, candidates: &[VectorCandidateRecord]) -> Result<(), CustomError> {
        let mut records = lock(&self.records)?;

        for candidate in candidates {
            records.retain(|record| {
                record.object_id != candidate.object_id || record.surface != candidate.surface
            });
            records.push(candidate.clone());
        }

        Ok(())
    }
}

#[async_trait]
impl VectorCandidateStore for FakeVectorCandidateStore {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        let candidates = records
            .iter()
            .map(|record| record.to_candidate_record())
            .collect::<Vec<_>>();
        self.replace_candidates(&candidates)
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
            .filter(|record| candidate_matches_filters(record, &query.filters))
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

fn candidate_matches_filters(
    record: &VectorCandidateRecord,
    filters: &VectorCandidateFilters,
) -> bool {
    (filters.retention_states.is_empty()
        || record
            .retention_state
            .is_some_and(|retention_state| filters.retention_states.contains(&retention_state)))
        && currentness_filters_match(record, filters)
        && ids_overlap(&filters.thread_ids, &record.relationship_hints.thread_ids)
        && ids_overlap(&filters.episode_ids, &record.relationship_hints.episode_ids)
        && entity_filter_matches(record, &filters.entity_ids)
        && filters
            .time_ranges
            .iter()
            .all(|time_range| time_range_matches(record, time_range))
}

fn currentness_filters_match(
    record: &VectorCandidateRecord,
    filters: &VectorCandidateFilters,
) -> bool {
    if !filters.has_currentness_filters() || record.object_type != ObjectType::DerivedMemory {
        return true;
    }

    filters
        .is_current
        .is_none_or(|is_current| record.is_current == Some(is_current))
        && filters
            .is_superseded
            .is_none_or(|is_superseded| record_is_superseded(record) == Some(is_superseded))
}

fn record_is_superseded(record: &VectorCandidateRecord) -> Option<bool> {
    record
        .payload_hints
        .is_superseded
        .or_else(|| record.is_current.map(|is_current| !is_current))
}

fn ids_overlap(filters: &[MemoryId], values: &[MemoryId]) -> bool {
    filters.is_empty() || filters.iter().any(|filter| values.contains(filter))
}

fn entity_filter_matches(record: &VectorCandidateRecord, entity_ids: &[MemoryId]) -> bool {
    entity_ids.is_empty()
        || entity_ids.iter().any(|entity_id| {
            record.relationship_hints.entity_ids.contains(entity_id)
                || record
                    .relationship_hints
                    .participant_entity_ids
                    .contains(entity_id)
                || record.relationship_hints.speaker_entity_id == Some(*entity_id)
        })
}

fn time_range_matches(record: &VectorCandidateRecord, time_range: &VectorTimeRangeFilter) -> bool {
    let value = match time_range.field {
        VectorTimeField::Created => record.payload_hints.created_at,
        VectorTimeField::Updated => record.payload_hints.updated_at,
        VectorTimeField::Started => record.payload_hints.started_at,
        VectorTimeField::Ended => record.payload_hints.ended_at,
        VectorTimeField::Observed => record.payload_hints.observed_at,
        VectorTimeField::LastTouched => record.payload_hints.last_touched_at,
    };

    value.is_some_and(|value| {
        time_range.after.is_none_or(|after| value >= after)
            && time_range.before.is_none_or(|before| value <= before)
    })
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
                (query.object_refs.is_empty()
                    || query.object_refs.iter().any(|object_ref| {
                        object_ref.object_id == object_id && object_ref.object_type == object_type
                    }))
                    && (query.object_ids.is_empty() || query.object_ids.contains(&object_id))
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
        let objects = lock(&self.objects)?.clone();
        let links = lock(&self.links)?.clone();
        bounded_expansion(query, objects, links)
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

    pub(crate) fn insert(
        &self,
        reference: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<(), CustomError> {
        let raw_reference = RawReference::new(reference, text);
        let mut entries = lock(&self.entries)?;
        entries.retain(|entry| entry.reference != raw_reference.reference);
        entries.push(raw_reference);
        Ok(())
    }

    pub(crate) fn insert_file(
        &self,
        reference: impl Into<String>,
        path: &Path,
    ) -> Result<(), CustomError> {
        let text = fs::read_to_string(path)
            .map_err(|error| CustomError::DatabaseError(error.to_string()))?;
        self.insert(reference, text)
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
        "Store contracts",
        Some("concept:store-contracts"),
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

pub(crate) fn high_fanout_graph_fixture() -> HighFanoutGraphFixture {
    let episode = simple_episode();
    let observation = salient_observation(episode.id, fixture_id(1));
    let hub_entity = entity(
        fixture_id(90),
        EntityType::Concept,
        "high fanout hub",
        Some("concept:high-fanout-hub"),
    );
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
                true,
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
    if left.len() != right.len() {
        return 0.0;
    }

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
    use crate::internal::models::vector::{VectorPayloadHints, VectorRelationshipHints};
    use crate::internal::repositories::{
        GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy,
        GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy, GraphObjectRef,
    };

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
    async fn vector_fake_upserts_full_records_through_store_contract() {
        let store = FakeVectorCandidateStore::new();
        let fixtures = representative_fixtures();
        let record = crate::internal::models::vector::episode_vector_record(&fixtures.episode);
        let records = vec![VectorRecordEmbedding::new(&record, &[1.0, 0.0])];

        store.upsert_vector_records(&records).await.unwrap();

        let matches = store
            .search_candidates(&VectorCandidateSearch::new(vec![1.0, 0.0], 10))
            .await
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].object_id, fixtures.episode.id);
        assert_eq!(matches[0].object_type, ObjectType::Episode);
        assert_eq!(matches[0].surface, VectorSurface::Summary);
    }

    #[tokio::test]
    async fn vector_fake_applies_payload_hint_prefilters_and_preserves_candidate_ordering() {
        let store = FakeVectorCandidateStore::new();
        let fixtures = representative_fixtures();
        let reflection = crate::internal::models::vector::derived_memory_vector_record(
            &fixtures.derived_reflection,
        );
        let preference = crate::internal::models::vector::derived_memory_vector_record(
            &fixtures.user_preference,
        );
        let suppressed = crate::internal::models::vector::derived_memory_vector_record(
            &fixtures.suppressed_seed,
        );

        store
            .upsert_vector_records(&[
                VectorRecordEmbedding::new(&reflection, &[0.8, 0.2]),
                VectorRecordEmbedding::new(&preference, &[1.0, 0.0]),
                VectorRecordEmbedding::new(&suppressed, &[1.0, 0.0]),
            ])
            .await
            .unwrap();

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_default_object_types()
            .with_filters(
                VectorCandidateFilters::new()
                    .with_retention_states(vec![RetentionState::Active])
                    .current_only()
                    .with_thread_ids(vec![fixtures.soft_thread.id])
                    .with_entity_ids(vec![fixtures.user_entity.id])
                    .with_episode_ids(vec![fixtures.episode.id])
                    .with_time_range(VectorTimeRangeFilter::new(
                        VectorTimeField::Updated,
                        Some(timestamp("2026-04-27T10:11:05Z")),
                        Some(timestamp("2026-04-27T10:11:07Z")),
                    )),
            );

        let matches = store.search_candidates(&query).await.unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].object_id, fixtures.user_preference.id);
        assert_eq!(matches[1].object_id, fixtures.derived_reflection.id);
        assert!(!matches
            .iter()
            .any(|matched| matched.object_id == fixtures.suppressed_seed.id));
    }

    #[tokio::test]
    async fn vector_fake_currentness_prefilter_keeps_non_derived_candidates() {
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
                    fixtures.suppressed_seed.id,
                    ObjectType::DerivedMemory,
                    VectorSurface::DerivedText,
                    vec![1.0, 0.0],
                )
                .with_filter_hints(
                    Some(RetentionState::Suppressed),
                    Some(false),
                    VectorRelationshipHints::default(),
                    VectorPayloadHints {
                        is_superseded: Some(true),
                        ..VectorPayloadHints::default()
                    },
                ),
            ])
            .await
            .unwrap();

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_default_object_types()
            .with_filters(VectorCandidateFilters::new().current_only());

        let matches = store.search_candidates(&query).await.unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].object_id, fixtures.episode.id);
        assert_eq!(matches[0].object_type, ObjectType::Episode);
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
    async fn graph_fake_queries_exact_typed_object_refs() {
        let store = FakeGraphAuthorityStore::new();
        let mut fixtures = representative_fixtures();
        fixtures.user_entity.id = fixtures.episode.id;

        store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Entity(fixtures.user_entity.clone()),
            ])
            .await
            .unwrap();

        let queried = store
            .query_objects(&GraphObjectQuery::by_refs(vec![GraphObjectRef::new(
                fixtures.episode.id,
                ObjectType::Episode,
            )]))
            .await
            .unwrap();

        assert_eq!(queried, vec![MemoryObject::Episode(fixtures.episode)]);
    }

    #[tokio::test]
    async fn graph_fake_honors_relation_allowlist_and_fanout_bounds() {
        let store = FakeGraphAuthorityStore::new();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_allowed_relation_types(vec![RelationType::About])
                    .with_max_fanout_per_node(3),
            )
            .await
            .unwrap();

        assert_eq!(expansion.objects.len(), 4);
        assert_eq!(expansion.links.len(), 3);
        assert_eq!(expansion.relations.len(), 3);
        assert!(expansion
            .objects
            .contains(&MemoryObject::Entity(fixture.hub_entity)));
        assert!(expansion.objects.iter().all(|object| matches!(
            object,
            MemoryObject::Entity(_) | MemoryObject::DerivedMemory(_)
        )));
        assert!(expansion
            .relations
            .iter()
            .all(|relation| relation.relation == RelationType::About && relation.proximity == 1));
    }

    #[tokio::test]
    async fn graph_fake_returns_only_traversed_links_after_fanout_pruning() {
        let store = FakeGraphAuthorityStore::new();
        let fixture = high_fanout_graph_fixture();
        let traversed_link = fixture.links[0].clone();
        let mut pruned_duplicate = traversed_link.clone();
        pruned_duplicate.id = fixture_id(195);
        pruned_duplicate.rationale = Some("Fanout-pruned duplicate endpoint link.".to_owned());
        let mut links = fixture.links.clone();
        links.push(pruned_duplicate.clone());

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&links).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_fanout_per_node(1),
            )
            .await
            .unwrap();

        assert_eq!(expansion.links, vec![traversed_link.clone()]);
        assert!(!expansion.links.contains(&pruned_duplicate));
        assert_eq!(
            expansion
                .relations
                .iter()
                .map(|relation| relation.link_id)
                .collect::<Vec<_>>(),
            vec![traversed_link.id]
        );
    }

    #[tokio::test]
    async fn graph_fake_reports_or_fails_closed_on_bounded_hub_policy() {
        let store = FakeGraphAuthorityStore::new();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let partial = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_hub_edges(2)
                    .with_max_fanout_per_node(2),
            )
            .await
            .unwrap();
        assert_eq!(
            partial.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::HubLimit
        );
        assert_eq!(partial.links.len(), 2);

        let fail_closed = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_hub_edges(2)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(250),
                        allow_partial_results: false,
                    }),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            fail_closed,
            CustomError::GraphExpansionBounded { reason, .. } if reason == "hub_limit"
        ));
    }

    #[tokio::test]
    async fn graph_fake_filters_lifecycle_currentness_and_superseded_nodes_by_default() {
        let store = FakeGraphAuthorityStore::new();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let default_expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.correction.id,
                ObjectType::DerivedMemory,
                1,
                5,
            ))
            .await
            .unwrap();

        assert_eq!(
            default_expansion.objects,
            vec![MemoryObject::DerivedMemory(fixtures.correction.clone())]
        );
        assert!(default_expansion.links.is_empty());
        assert_eq!(default_expansion.filtered_nodes.len(), 1);
        assert_eq!(
            default_expansion.filtered_nodes[0].object_ref,
            GraphObjectRef::new(fixtures.suppressed_seed.id, ObjectType::DerivedMemory)
        );
        assert_eq!(
            default_expansion.filtered_nodes[0].reason,
            GraphExpansionFilteredReason::Suppressed
        );

        let historical_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.correction.id, ObjectType::DerivedMemory, 1, 5)
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                        include_suppressed: true,
                        include_non_current: true,
                        include_superseded: true,
                        ..GraphExpansionLifecyclePolicy::default()
                    }),
            )
            .await
            .unwrap();

        assert!(historical_expansion
            .objects
            .contains(&MemoryObject::DerivedMemory(
                fixtures.suppressed_seed.clone()
            )));
        assert_eq!(
            historical_expansion.links,
            vec![fixtures.hub_links[2].clone()]
        );
        assert!(historical_expansion.filtered_nodes.is_empty());
    }

    #[tokio::test]
    async fn graph_fake_uses_deterministic_timeout_substitute() {
        let store = FakeGraphAuthorityStore::new();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 5)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(0),
                        allow_partial_results: true,
                    }),
            )
            .await
            .unwrap();

        assert!(expansion.objects.is_empty());
        assert_eq!(
            expansion.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::Timeout
        );
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
    fn representative_fixtures_cover_canonical_memory_scenarios() {
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

    #[test]
    fn cosine_similarity_rejects_mismatched_dimensions() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
    }
}
