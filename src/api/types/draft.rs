use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::write_plan::{RememberDiagnostics, RepairMarker, StatsUpdateStatus};
use crate::domain::{
    BeliefAssertion, DerivedMemory, DerivedType, DomainValidationError, Entity, Episode, MemoryId,
    MemoryLink, MemoryObject, MemoryObjectRef, MemoryThread, Modality, ObjectType, Observation,
    RelationType, RetentionState, Scene, ThreadStatus, DEFAULT_SCHEMA_VERSION,
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

/// Caller-supplied identity for a notion the character holds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EntityDraft {
    pub id: Option<MemoryId>,
    pub created_at: Option<DateTime<Utc>>,
    pub schema_version: Option<String>,
}

impl EntityDraft {
    pub fn new() -> Self {
        Self::default()
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
            created_at,
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
    /// Overrides the input scene; a caller-built episode candidate must supply it.
    pub scene: Option<Scene>,
    pub ended_at: Option<DateTime<Utc>>,
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
            scene: None,
            ended_at: None,
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
        let scene = self
            .scene
            .ok_or(DomainValidationError::MissingScene)?
            .without_blank_participants();
        let episode = Episode {
            id: defaults.id(self.id),
            object_type: ObjectType::Episode,
            modality: self.modality,
            scene,
            ended_at: self.ended_at,
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
    /// The notions this interpreted memory is about (its subjects).
    pub entity_ids: Vec<MemoryId>,
    /// The character's commitments about subjects in `entity_ids`.
    pub assertions: Vec<BeliefAssertion>,
    /// Source-free grounding given by the application; requires at least one notion subject.
    pub given_by_application: bool,
    pub salience_score: f32,
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
            assertions: Vec::new(),
            given_by_application: false,
            salience_score: 0.5,
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
        let mut derived = DerivedMemory {
            id: defaults.id(self.id),
            object_type: ObjectType::DerivedMemory,
            derived_type: self.derived_type,
            text: self.text,
            derived_from_episode_ids: self.derived_from_episode_ids,
            derived_from_observation_ids: self.derived_from_observation_ids,
            thread_ids: self.thread_ids,
            entity_ids: self.entity_ids,
            scope_keys: Vec::new(),
            assertions: self.assertions,
            given_by_application: self.given_by_application,
            salience_score: self.salience_score,
            supersedes: self.supersedes,
            retention_state: self.retention_state,
            created_at,
            updated_at: self.updated_at.unwrap_or(created_at),
            schema_version: defaults.schema_version(self.schema_version),
        };
        for ids in [
            &mut derived.derived_from_episode_ids,
            &mut derived.derived_from_observation_ids,
            &mut derived.thread_ids,
            &mut derived.entity_ids,
            &mut derived.supersedes,
        ] {
            ids.sort_unstable();
            ids.dedup();
        }
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
/// Supersedes links and About links between interpreted memories and entities
/// are derived from the memory's lists and cannot be authored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryLinkDraft {
    pub id: Option<MemoryId>,
    pub from_id: MemoryId,
    pub from_type: ObjectType,
    pub to_id: MemoryId,
    pub to_type: ObjectType,
    pub relation: RelationType,
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
        if self.relation == RelationType::Supersedes {
            return Err(DomainValidationError::AuthoredSupersedesLink);
        }
        if self.relation == RelationType::About
            && matches!(
                (self.from_type, self.to_type),
                (ObjectType::DerivedMemory, ObjectType::Entity)
                    | (ObjectType::Entity, ObjectType::DerivedMemory)
            )
        {
            return Err(DomainValidationError::AuthoredBeliefAboutLink);
        }
        let link = MemoryLink {
            id: defaults.id(self.id),
            object_type: ObjectType::MemoryLink,
            from_id: self.from_id,
            from_type: self.from_type,
            to_id: self.to_id,
            to_type: self.to_type,
            relation: self.relation,
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

/// Result of a graph-authoritative link write and its repairable stats projection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkOutcome {
    pub link: MemoryLink,
    pub stats_update_status: StatsUpdateStatus,
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

        let entity = EntityDraft::new()
            .into_domain_with_defaults(&mut defaults)
            .unwrap();

        assert_eq!(entity.id, id);
        assert_eq!(entity.created_at, now);
        assert_eq!(entity.schema_version, DEFAULT_SCHEMA_VERSION);
        assert_eq!(entity.object_type, ObjectType::Entity);
    }

    #[test]
    fn caller_supplied_values_are_preserved() {
        let id = memory_id("550e8400-e29b-41d4-a716-446655441010");
        let created_at = timestamp("2026-04-28T12:01:00Z");
        let mut draft = EpisodeDraft::new("Discussed durable draft inputs.");
        draft.id = Some(id);
        let mut scene = Scene::at((created_at).fixed_offset());
        scene.setting.key = Some("conversation-42".to_owned());
        draft.scene = Some(scene);
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
        episode.scene = Some(Scene::now());
        assert_eq!(
            episode.into_domain(),
            Err(DomainValidationError::EmptyEpisodeSummary)
        );
    }

    #[test]
    fn assistant_preference_is_the_public_serialized_derived_type_name() {
        let serialized = serde_json::to_string(&DerivedType::AssistantPreference).unwrap();
        assert_eq!(serialized, "\"assistant_preference\"");
    }
}
