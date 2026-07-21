use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::write_plan::{RememberDiagnostics, RepairMarker, StatsUpdateStatus};
use crate::domain::{
    DerivedMemory, DerivedType, DomainValidationError, Entity, EntityType, Episode, MemoryId,
    MemoryLink, MemoryObject, MemoryObjectRef, MemoryThread, Modality, ObjectType, Observation,
    RelationType, RetentionState, Stability, ThreadStatus, DEFAULT_SCHEMA_VERSION,
};
use crate::errors::VectorIndexingCause;

/// Supplies generated IDs and timestamps for converting draft inputs into canonical objects.
#[derive(Debug, Clone)]
pub struct DraftDefaults {
    now: DateTime<Utc>,
    ids: VecDeque<MemoryId>,
}

impl DraftDefaults {
    pub fn generated() -> Self {
        Self {
            now: Utc::now(),
            ids: VecDeque::new(),
        }
    }

    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            now,
            ids: VecDeque::new(),
        }
    }

    pub fn with_id_sequence(now: DateTime<Utc>, ids: impl IntoIterator<Item = MemoryId>) -> Self {
        Self {
            now,
            ids: ids.into_iter().collect(),
        }
    }

    fn id(&mut self, supplied: Option<MemoryId>) -> MemoryId {
        supplied.unwrap_or_else(|| self.ids.pop_front().unwrap_or_else(uuid::Uuid::new_v4))
    }

    fn timestamp(&self, supplied: Option<DateTime<Utc>>) -> DateTime<Utc> {
        supplied.unwrap_or(self.now)
    }

    fn schema_version(&self, supplied: Option<String>) -> String {
        supplied.unwrap_or_else(|| DEFAULT_SCHEMA_VERSION.to_owned())
    }
}

impl Default for DraftDefaults {
    fn default() -> Self {
        Self::generated()
    }
}

/// Caller-supplied draft for a canonical entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityDraft {
    pub id: Option<MemoryId>,
    pub entity_type: EntityType,
    pub name: String,
    pub aliases: Vec<String>,
    pub canonical_key: Option<String>,
    pub summary: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl EntityDraft {
    pub fn new(entity_type: EntityType, name: impl Into<String>) -> Self {
        Self {
            id: None,
            entity_type,
            name: name.into(),
            aliases: Vec::new(),
            canonical_key: None,
            summary: None,
            created_at: None,
            updated_at: None,
            schema_version: None,
        }
    }

    pub fn into_domain(self) -> Result<Entity, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<Entity, DomainValidationError> {
        let created_at = defaults.timestamp(self.created_at);
        let entity = Entity {
            id: defaults.id(self.id),
            object_type: ObjectType::Entity,
            entity_type: self.entity_type,
            name: self.name,
            aliases: self.aliases,
            canonical_key: self.canonical_key,
            summary: self.summary,
            created_at,
            updated_at: self.updated_at.unwrap_or(created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        entity.validate()?;
        Ok(entity)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::Entity)
    }
}

impl TryFrom<EntityDraft> for Entity {
    type Error = DomainValidationError;

    fn try_from(value: EntityDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Caller-supplied draft for an episode with an external raw reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeDraft {
    pub id: Option<MemoryId>,
    pub modality: Modality,
    pub source_conversation_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub participant_entity_ids: Vec<MemoryId>,
    pub summary: String,
    pub raw_ref: Option<String>,
    pub salience_score: f32,
    pub retention_state: RetentionState,
    pub created_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl EpisodeDraft {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            id: None,
            modality: Modality::Chat,
            source_conversation_id: None,
            started_at: None,
            ended_at: None,
            participant_entity_ids: Vec::new(),
            summary: summary.into(),
            raw_ref: None,
            salience_score: 0.5,
            retention_state: RetentionState::Active,
            created_at: None,
            schema_version: None,
        }
    }

    pub fn into_domain(self) -> Result<Episode, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<Episode, DomainValidationError> {
        let episode = Episode {
            id: defaults.id(self.id),
            object_type: ObjectType::Episode,
            modality: self.modality,
            source_conversation_id: self.source_conversation_id,
            started_at: self.started_at,
            ended_at: self.ended_at,
            participant_entity_ids: self.participant_entity_ids,
            summary: self.summary,
            raw_ref: self.raw_ref,
            salience_score: self.salience_score,
            retention_state: self.retention_state,
            created_at: defaults.timestamp(self.created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        episode.validate()?;
        Ok(episode)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::Episode)
    }
}

impl TryFrom<EpisodeDraft> for Episode {
    type Error = DomainValidationError;

    fn try_from(value: EpisodeDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Caller-supplied draft for an observation with an external raw reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationDraft {
    pub id: Option<MemoryId>,
    pub episode_id: MemoryId,
    pub speaker_entity_id: Option<MemoryId>,
    pub observed_at: Option<DateTime<Utc>>,
    pub modality: Modality,
    pub text: String,
    pub raw_ref: Option<String>,
    pub salience_score: f32,
    pub retention_state: RetentionState,
    pub created_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl ObservationDraft {
    pub fn new(episode_id: MemoryId, text: impl Into<String>) -> Self {
        Self {
            id: None,
            episode_id,
            speaker_entity_id: None,
            observed_at: None,
            modality: Modality::Chat,
            text: text.into(),
            raw_ref: None,
            salience_score: 0.5,
            retention_state: RetentionState::Active,
            created_at: None,
            schema_version: None,
        }
    }

    pub fn into_domain(self) -> Result<Observation, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<Observation, DomainValidationError> {
        let observation = Observation {
            id: defaults.id(self.id),
            object_type: ObjectType::Observation,
            episode_id: self.episode_id,
            speaker_entity_id: self.speaker_entity_id,
            observed_at: self.observed_at,
            modality: self.modality,
            text: self.text,
            raw_ref: self.raw_ref,
            salience_score: self.salience_score,
            retention_state: self.retention_state,
            created_at: defaults.timestamp(self.created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::Observation)
    }
}

impl TryFrom<ObservationDraft> for Observation {
    type Error = DomainValidationError;

    fn try_from(value: ObservationDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Caller-supplied draft for a memory thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryThreadDraft {
    pub id: Option<MemoryId>,
    pub title: String,
    pub summary: String,
    pub status: ThreadStatus,
    pub last_touched_at: Option<DateTime<Utc>>,
    pub salience_score: f32,
    pub canonical_key: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl MemoryThreadDraft {
    pub fn new(title: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            summary: summary.into(),
            status: ThreadStatus::Active,
            last_touched_at: None,
            salience_score: 0.5,
            canonical_key: None,
            created_at: None,
            updated_at: None,
            schema_version: None,
        }
    }

    pub fn into_domain(self) -> Result<MemoryThread, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<MemoryThread, DomainValidationError> {
        let created_at = defaults.timestamp(self.created_at);
        let updated_at = self.updated_at.unwrap_or(created_at);
        let thread = MemoryThread {
            id: defaults.id(self.id),
            object_type: ObjectType::MemoryThread,
            title: self.title,
            summary: self.summary,
            status: self.status,
            last_touched_at: self.last_touched_at.unwrap_or(updated_at),
            salience_score: self.salience_score,
            canonical_key: self.canonical_key,
            created_at,
            updated_at,
            schema_version: defaults.schema_version(self.schema_version),
        };
        thread.validate()?;
        Ok(thread)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::MemoryThread)
    }
}

impl TryFrom<MemoryThreadDraft> for MemoryThread {
    type Error = DomainValidationError;

    fn try_from(value: MemoryThreadDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Caller-supplied draft for a derived memory and its source references.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivedMemoryDraft {
    pub id: Option<MemoryId>,
    pub derived_type: DerivedType,
    pub text: String,
    pub derived_from_episode_ids: Vec<MemoryId>,
    pub derived_from_observation_ids: Vec<MemoryId>,
    pub thread_ids: Vec<MemoryId>,
    pub entity_ids: Vec<MemoryId>,
    pub confidence: f32,
    pub salience_score: f32,
    pub stability: Stability,
    pub is_current: bool,
    pub supersedes: Vec<MemoryId>,
    pub retention_state: RetentionState,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl DerivedMemoryDraft {
    pub fn new(derived_type: DerivedType, text: impl Into<String>) -> Self {
        Self {
            id: None,
            derived_type,
            text: text.into(),
            derived_from_episode_ids: Vec::new(),
            derived_from_observation_ids: Vec::new(),
            thread_ids: Vec::new(),
            entity_ids: Vec::new(),
            confidence: 1.0,
            salience_score: 0.5,
            stability: Stability::Medium,
            is_current: true,
            supersedes: Vec::new(),
            retention_state: RetentionState::Active,
            created_at: None,
            updated_at: None,
            schema_version: None,
        }
    }

    pub fn with_source_episode(mut self, episode_id: MemoryId) -> Self {
        self.derived_from_episode_ids.push(episode_id);
        self
    }

    pub fn with_source_observation(mut self, observation_id: MemoryId) -> Self {
        self.derived_from_observation_ids.push(observation_id);
        self
    }

    pub fn into_domain(self) -> Result<DerivedMemory, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<DerivedMemory, DomainValidationError> {
        let created_at = defaults.timestamp(self.created_at);
        let derived = DerivedMemory {
            id: defaults.id(self.id),
            object_type: ObjectType::DerivedMemory,
            derived_type: self.derived_type,
            text: self.text,
            derived_from_episode_ids: self.derived_from_episode_ids,
            derived_from_observation_ids: self.derived_from_observation_ids,
            thread_ids: self.thread_ids,
            entity_ids: self.entity_ids,
            confidence: self.confidence,
            salience_score: self.salience_score,
            stability: self.stability,
            is_current: self.is_current,
            supersedes: self.supersedes,
            retention_state: self.retention_state,
            created_at,
            updated_at: self.updated_at.unwrap_or(created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        derived.validate()?;
        Ok(derived)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::DerivedMemory)
    }
}

impl TryFrom<DerivedMemoryDraft> for DerivedMemory {
    type Error = DomainValidationError;

    fn try_from(value: DerivedMemoryDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Caller-supplied draft for a canonical typed memory link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryLinkDraft {
    pub id: Option<MemoryId>,
    pub from_id: MemoryId,
    pub from_type: ObjectType,
    pub to_id: MemoryId,
    pub to_type: ObjectType,
    pub relation: RelationType,
    pub confidence: f32,
    pub rationale: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl MemoryLinkDraft {
    pub fn new(
        from_type: ObjectType,
        from_id: MemoryId,
        relation: RelationType,
        to_type: ObjectType,
        to_id: MemoryId,
    ) -> Self {
        Self {
            id: None,
            from_id,
            from_type,
            to_id,
            to_type,
            relation,
            confidence: 1.0,
            rationale: None,
            created_at: None,
            schema_version: None,
        }
    }

    pub fn into_domain(self) -> Result<MemoryLink, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<MemoryLink, DomainValidationError> {
        let link = MemoryLink {
            id: defaults.id(self.id),
            object_type: ObjectType::MemoryLink,
            from_id: self.from_id,
            from_type: self.from_type,
            to_id: self.to_id,
            to_type: self.to_type,
            relation: self.relation,
            confidence: self.confidence,
            rationale: self.rationale,
            created_at: defaults.timestamp(self.created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        link.validate()?;
        Ok(link)
    }

    pub fn into_memory_object(self) -> Result<MemoryObject, DomainValidationError> {
        self.into_domain().map(MemoryObject::MemoryLink)
    }
}

impl TryFrom<MemoryLinkDraft> for MemoryLink {
    type Error = DomainValidationError;

    fn try_from(value: MemoryLinkDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

/// Result of a remember write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberOutcome {
    pub persisted_object_ids: Vec<MemoryId>,
    pub persisted_link_ids: Vec<MemoryId>,
    pub vector_indexed_object_ids: Vec<MemoryId>,
    pub vector_indexing_failure: Option<VectorIndexingFailure>,
    pub stats_update_status: StatsUpdateStatus,
    pub repair_needed: Vec<RepairMarker>,
    pub diagnostics: RememberDiagnostics,
}

/// Vector indexing failure recorded after graph-authoritative writes have succeeded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorIndexingFailure {
    pub unindexed_objects: Vec<MemoryObjectRef>,
    pub cause: VectorIndexingCause,
}

impl VectorIndexingFailure {
    pub fn unindexed_object_ids(&self) -> Vec<MemoryId> {
        self.unindexed_objects
            .iter()
            .map(|object| object.id)
            .collect()
    }
}

/// Draft wrapper for converting caller input into a canonical memory object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "object_type", content = "object", rename_all = "snake_case")]
pub enum MemoryObjectDraft {
    Episode(EpisodeDraft),
    Observation(ObservationDraft),
    Entity(EntityDraft),
    MemoryThread(MemoryThreadDraft),
    DerivedMemory(DerivedMemoryDraft),
    MemoryLink(MemoryLinkDraft),
}

impl MemoryObjectDraft {
    pub fn into_domain(self) -> Result<MemoryObject, DomainValidationError> {
        let mut defaults = DraftDefaults::generated();
        self.into_domain_with_defaults(&mut defaults)
    }

    pub fn into_domain_with_defaults(
        self,
        defaults: &mut DraftDefaults,
    ) -> Result<MemoryObject, DomainValidationError> {
        match self {
            Self::Episode(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::Episode),
            Self::Observation(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::Observation),
            Self::Entity(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::Entity),
            Self::MemoryThread(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::MemoryThread),
            Self::DerivedMemory(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::DerivedMemory),
            Self::MemoryLink(draft) => draft
                .into_domain_with_defaults(defaults)
                .map(MemoryObject::MemoryLink),
        }
    }
}

impl TryFrom<MemoryObjectDraft> for MemoryObject {
    type Error = DomainValidationError;

    fn try_from(value: MemoryObjectDraft) -> Result<Self, Self::Error> {
        value.into_domain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    fn memory_id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn draft_defaults_supply_stable_ids_timestamps_and_schema_version() {
        let now = timestamp("2026-04-28T12:00:00Z");
        let id = memory_id("550e8400-e29b-41d4-a716-446655441001");
        let mut defaults = DraftDefaults::with_id_sequence(now, [id]);

        let entity = EntityDraft::new(EntityType::User, "Kohta")
            .into_domain_with_defaults(&mut defaults)
            .unwrap();

        assert_eq!(entity.id, id);
        assert_eq!(entity.created_at, now);
        assert_eq!(entity.updated_at, now);
        assert_eq!(entity.schema_version, DEFAULT_SCHEMA_VERSION);
        assert_eq!(entity.object_type, ObjectType::Entity);
    }

    #[test]
    fn caller_supplied_values_are_preserved() {
        let id = memory_id("550e8400-e29b-41d4-a716-446655441010");
        let created_at = timestamp("2026-04-28T12:01:00Z");
        let mut draft = EpisodeDraft::new("Discussed durable draft inputs.");
        draft.id = Some(id);
        draft.source_conversation_id = Some("conversation-42".to_owned());
        draft.raw_ref = Some("raw://conversation/42#episode".to_owned());
        draft.salience_score = 0.8;
        draft.created_at = Some(created_at);
        draft.schema_version = Some("test_schema".to_owned());

        let episode = draft.into_domain().unwrap();

        assert_eq!(episode.id, id);
        assert_eq!(episode.created_at, created_at);
        assert_eq!(
            episode.raw_ref.as_deref(),
            Some("raw://conversation/42#episode")
        );
        assert_eq!(episode.schema_version, "test_schema");
        assert_eq!(episode.salience_score, 0.8);
    }

    #[test]
    fn draft_conversions_cover_all_canonical_object_variants() {
        let now = timestamp("2026-04-28T12:02:00Z");
        let ids = [
            memory_id("550e8400-e29b-41d4-a716-446655441020"),
            memory_id("550e8400-e29b-41d4-a716-446655441021"),
            memory_id("550e8400-e29b-41d4-a716-446655441022"),
            memory_id("550e8400-e29b-41d4-a716-446655441023"),
            memory_id("550e8400-e29b-41d4-a716-446655441024"),
            memory_id("550e8400-e29b-41d4-a716-446655441025"),
        ];
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655441030");
        let observation_id = memory_id("550e8400-e29b-41d4-a716-446655441031");
        let mut defaults = DraftDefaults::with_id_sequence(now, ids);

        let drafts = [
            MemoryObjectDraft::Episode(EpisodeDraft::new("Episode summary")),
            MemoryObjectDraft::Observation(ObservationDraft::new(episode_id, "Observation text")),
            MemoryObjectDraft::Entity(EntityDraft::new(EntityType::Concept, "Drafts")),
            MemoryObjectDraft::MemoryThread(MemoryThreadDraft::new("Thread", "Thread summary")),
            MemoryObjectDraft::DerivedMemory(
                DerivedMemoryDraft::new(
                    DerivedType::AssistantPreference,
                    "Assistant prefers concise context.",
                )
                .with_source_episode(episode_id),
            ),
            MemoryObjectDraft::MemoryLink(MemoryLinkDraft::new(
                ObjectType::DerivedMemory,
                ids[4],
                RelationType::DerivedFrom,
                ObjectType::Observation,
                observation_id,
            )),
        ];

        let objects = drafts
            .into_iter()
            .map(|draft| draft.into_domain_with_defaults(&mut defaults).unwrap())
            .collect::<Vec<_>>();

        assert!(matches!(objects[0], MemoryObject::Episode(_)));
        assert!(matches!(objects[1], MemoryObject::Observation(_)));
        assert!(matches!(objects[2], MemoryObject::Entity(_)));
        assert!(matches!(objects[3], MemoryObject::MemoryThread(_)));
        assert!(matches!(objects[4], MemoryObject::DerivedMemory(_)));
        assert!(matches!(objects[5], MemoryObject::MemoryLink(_)));
    }

    #[test]
    fn derived_memory_draft_requires_episode_or_observation_source() {
        let error = DerivedMemoryDraft::new(DerivedType::Reflection, "No source")
            .into_domain()
            .unwrap_err();

        assert_eq!(error, DomainValidationError::MissingDerivedSource);
    }

    #[test]
    fn draft_validation_reuses_domain_validation_errors() {
        let mut episode = EpisodeDraft::new(" ");
        episode.salience_score = 0.5;
        assert_eq!(
            episode.into_domain(),
            Err(DomainValidationError::EmptyEpisodeSummary)
        );

        let mut link = MemoryLinkDraft::new(
            ObjectType::Episode,
            memory_id("550e8400-e29b-41d4-a716-446655441040"),
            RelationType::Mentions,
            ObjectType::Entity,
            memory_id("550e8400-e29b-41d4-a716-446655441041"),
        );
        link.confidence = f32::NAN;
        assert!(matches!(
            link.into_domain(),
            Err(DomainValidationError::InvalidScore {
                field: "MemoryLink.confidence",
                ..
            })
        ));
    }

    #[test]
    fn assistant_preference_is_the_public_serialized_derived_type_name() {
        let serialized = serde_json::to_string(&DerivedType::AssistantPreference).unwrap();
        assert_eq!(serialized, "\"assistant_preference\"");
    }
}
