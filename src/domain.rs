mod lifecycle;
mod object_ref;
mod retrieval;
pub(crate) mod schema;
mod write_validation;

pub use lifecycle::LifecycleDtoValidationError;
pub use object_ref::MemoryObjectRef;
pub use retrieval::{GraphExpansionBoundedFailureTrace, GraphExpansionBoundedReason};
pub use write_validation::{
    CandidateProvenanceIssue, CandidateReferenceRole, CandidateScoreField,
    CandidateSourceSpanIssue, CandidateTimestampField, CandidateValidation,
    CandidateValidationIssue, CandidateValidationStatus, MemoryCandidateKind, MemoryLinkEndpoint,
    PlanIdentityField,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type MemoryId = uuid::Uuid;

pub const EPISODIC_MEMORY_SCHEMA_VERSION: &str = "episodic_memory_initial";
pub const CURRENT_SCHEMA_VERSION: &str = EPISODIC_MEMORY_SCHEMA_VERSION;
pub const DEFAULT_SCHEMA_VERSION: &str = CURRENT_SCHEMA_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Episode,
    Observation,
    Entity,
    MemoryThread,
    DerivedMemory,
    MemoryLink,
}

impl ObjectType {
    pub const fn graph_segment(self) -> &'static str {
        match self {
            Self::Episode => "episode",
            Self::Observation => "observation",
            Self::Entity => "entity",
            Self::MemoryThread => "thread",
            Self::DerivedMemory => "derived-memory",
            Self::MemoryLink => "link",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Chat,
    VoiceTranscript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    User,
    Assistant,
    Project,
    Concept,
    Tool,
    Document,
    Place,
    Organization,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedType {
    Reflection,
    UserPreference,
    AssistantPreference,
    Commitment,
    OpenLoop,
    CharacterSignal,
    RelationshipNote,
    ProjectNote,
    Claim,
    Correction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    HasObservation,
    ObservedIn,
    Mentions,
    Involves,
    About,
    DerivedFrom,
    PartOfThread,
    Supports,
    Contradicts,
    Supersedes,
    Resolves,
    CreatesOpenLoop,
    FulfillsCommitment,
    AssociatedWith,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionState {
    Active,
    Suppressed,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Active,
    Dormant,
    Resolved,
    Archived,
}

pub fn graph_uri(object_type: ObjectType, id: MemoryId) -> String {
    format!("urn:cmem:{}:{}", object_type.graph_segment(), id)
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainValidationError {
    #[error("{field} object_type must be {expected:?}, got {actual:?}")]
    ObjectTypeMismatch {
        field: &'static str,
        expected: ObjectType,
        actual: ObjectType,
    },

    #[error("episode summary must not be empty")]
    EmptyEpisodeSummary,

    #[error("observation episode_id must reference an episode")]
    MissingEpisodeReference,

    #[error("derived memory must reference at least one source episode or observation")]
    MissingDerivedSource,

    #[error("{field} must be in 0.0..=1.0 and finite, got {value}")]
    InvalidScore { field: &'static str, value: f32 },

    #[error("memory links cannot point at MemoryLink endpoints via {field}")]
    UnsupportedMemoryLinkEndpoint { field: &'static str },

    #[error("memory links cannot point from an object to itself: {object_type:?} {id}")]
    SelfLink {
        object_type: ObjectType,
        id: MemoryId,
    },
}

fn validate_object_type(
    field: &'static str,
    actual: ObjectType,
    expected: ObjectType,
) -> Result<(), DomainValidationError> {
    if actual != expected {
        return Err(DomainValidationError::ObjectTypeMismatch {
            field,
            expected,
            actual,
        });
    }

    Ok(())
}

fn validate_score(field: &'static str, value: f32) -> Result<(), DomainValidationError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DomainValidationError::InvalidScore { field, value });
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Episode {
    pub id: MemoryId,
    pub object_type: ObjectType,
    pub modality: Modality,
    pub source_conversation_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub participant_entity_ids: Vec<MemoryId>,
    pub summary: String,
    pub raw_ref: Option<String>,
    pub salience_score: f32,
    pub retention_state: RetentionState,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
}

impl Episode {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type("Episode.object_type", self.object_type, ObjectType::Episode)?;
        if self.summary.trim().is_empty() {
            return Err(DomainValidationError::EmptyEpisodeSummary);
        }
        validate_score("Episode.salience_score", self.salience_score)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub id: MemoryId,
    pub object_type: ObjectType,
    pub episode_id: MemoryId,
    pub speaker_entity_id: Option<MemoryId>,
    pub observed_at: Option<DateTime<Utc>>,
    pub modality: Modality,
    pub text: String,
    pub raw_ref: Option<String>,
    pub salience_score: f32,
    pub retention_state: RetentionState,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
}

impl Observation {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type(
            "Observation.object_type",
            self.object_type,
            ObjectType::Observation,
        )?;
        if self.episode_id.is_nil() {
            return Err(DomainValidationError::MissingEpisodeReference);
        }
        validate_score("Observation.salience_score", self.salience_score)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub id: MemoryId,
    pub object_type: ObjectType,
    pub entity_type: EntityType,
    pub name: String,
    pub aliases: Vec<String>,
    pub canonical_key: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: String,
}

impl Entity {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type("Entity.object_type", self.object_type, ObjectType::Entity)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryThread {
    pub id: MemoryId,
    pub object_type: ObjectType,
    pub title: String,
    pub summary: String,
    pub status: ThreadStatus,
    pub last_touched_at: DateTime<Utc>,
    pub salience_score: f32,
    pub canonical_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: String,
}

impl MemoryThread {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type(
            "MemoryThread.object_type",
            self.object_type,
            ObjectType::MemoryThread,
        )?;
        validate_score("MemoryThread.salience_score", self.salience_score)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivedMemory {
    pub id: MemoryId,
    pub object_type: ObjectType,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: String,
}

impl DerivedMemory {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type(
            "DerivedMemory.object_type",
            self.object_type,
            ObjectType::DerivedMemory,
        )?;
        if self.derived_from_episode_ids.is_empty() && self.derived_from_observation_ids.is_empty()
        {
            return Err(DomainValidationError::MissingDerivedSource);
        }
        validate_score("DerivedMemory.confidence", self.confidence)?;
        validate_score("DerivedMemory.salience_score", self.salience_score)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryLink {
    pub id: MemoryId,
    pub object_type: ObjectType,
    pub from_id: MemoryId,
    pub from_type: ObjectType,
    pub to_id: MemoryId,
    pub to_type: ObjectType,
    pub relation: RelationType,
    pub confidence: f32,
    pub rationale: Option<String>,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
}

impl MemoryLink {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        validate_object_type(
            "MemoryLink.object_type",
            self.object_type,
            ObjectType::MemoryLink,
        )?;
        validate_score("MemoryLink.confidence", self.confidence)?;
        validate_link_endpoint("MemoryLink.from_type", self.from_type)?;
        validate_link_endpoint("MemoryLink.to_type", self.to_type)?;

        if self.from_id == self.to_id && self.from_type == self.to_type {
            return Err(DomainValidationError::SelfLink {
                object_type: self.from_type,
                id: self.from_id,
            });
        }

        Ok(())
    }
}

fn validate_link_endpoint(
    field: &'static str,
    object_type: ObjectType,
) -> Result<(), DomainValidationError> {
    if object_type == ObjectType::MemoryLink {
        return Err(DomainValidationError::UnsupportedMemoryLinkEndpoint { field });
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "object_type", content = "object", rename_all = "snake_case")]
pub enum MemoryObject {
    Episode(Episode),
    Observation(Observation),
    Entity(Entity),
    MemoryThread(MemoryThread),
    DerivedMemory(DerivedMemory),
    MemoryLink(MemoryLink),
}

impl MemoryObject {
    pub fn validate(&self) -> Result<(), DomainValidationError> {
        match self {
            Self::Episode(object) => object.validate(),
            Self::Observation(object) => object.validate(),
            Self::Entity(object) => object.validate(),
            Self::MemoryThread(object) => object.validate(),
            Self::DerivedMemory(object) => object.validate(),
            Self::MemoryLink(object) => object.validate(),
        }
    }
}

#[cfg(test)]
mod tests;
