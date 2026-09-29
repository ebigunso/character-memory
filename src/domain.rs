pub(crate) mod belief;
mod lifecycle;
mod object_ref;
mod retrieval;
mod scene;
pub(crate) mod schema;
mod write_validation;

pub use belief::{BeliefAssertion, BeliefPredicate, BeliefValidationError};
pub use lifecycle::{LifecycleDtoValidationError, SourceReferenceKind};
pub use object_ref::MemoryObjectRef;
pub use retrieval::{GraphExpansionBoundedFailureTrace, GraphExpansionBoundedReason};
pub(crate) use scene::ScopeKey;
pub use scene::{Scene, SceneParticipant, SceneSetting};
pub use write_validation::{
    CandidateProvenanceIssue, CandidateReferenceRole, CandidateScoreField,
    CandidateSourceSpanIssue, CandidateTimestampField, CandidateValidation,
    CandidateValidationIssue, CandidateValidationStatus, MemoryCandidateKind, MemoryLinkEndpoint,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

pub type MemoryId = uuid::Uuid;

pub const DEFAULT_SCHEMA_VERSION: &str = "episodic_memory_initial";

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

impl fmt::Display for ObjectType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Episode => "episode",
            Self::Observation => "observation",
            Self::Entity => "entity",
            Self::MemoryThread => "memory_thread",
            Self::DerivedMemory => "derived_memory",
            Self::MemoryLink => "memory_link",
        })
    }
}

impl FromStr for ObjectType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "episode" => Ok(Self::Episode),
            "observation" => Ok(Self::Observation),
            "entity" => Ok(Self::Entity),
            "memory_thread" => Ok(Self::MemoryThread),
            "derived_memory" => Ok(Self::DerivedMemory),
            "memory_link" => Ok(Self::MemoryLink),
            _ => Err(format!("unknown object type token: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorSurface {
    Summary,
    Text,
    DerivedText,
    SceneSetting,
    SceneParticipants,
    Query,
}

impl fmt::Display for VectorSurface {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Summary => "summary",
            Self::Text => "text",
            Self::DerivedText => "derived_text",
            Self::SceneSetting => "scene_setting",
            Self::SceneParticipants => "scene_participants",
            Self::Query => "query",
        })
    }
}

impl FromStr for VectorSurface {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "summary" => Ok(Self::Summary),
            "text" => Ok(Self::Text),
            "derived_text" => Ok(Self::DerivedText),
            "scene_setting" => Ok(Self::SceneSetting),
            "scene_participants" => Ok(Self::SceneParticipants),
            "query" => Ok(Self::Query),
            _ => Err(format!("unknown vector surface token: {value}")),
        }
    }
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

    pub(crate) const fn stable_rank(self) -> u8 {
        match self {
            Self::Episode => 0,
            Self::Observation => 1,
            Self::Entity => 2,
            Self::MemoryThread => 3,
            Self::DerivedMemory => 4,
            Self::MemoryLink => 5,
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

impl RelationType {
    pub(crate) const fn stable_rank(self) -> u8 {
        match self {
            Self::HasObservation => 0,
            Self::ObservedIn => 1,
            Self::Mentions => 2,
            Self::Involves => 3,
            Self::About => 4,
            Self::DerivedFrom => 5,
            Self::PartOfThread => 6,
            Self::Supports => 7,
            Self::Contradicts => 8,
            Self::Supersedes => 9,
            Self::Resolves => 10,
            Self::CreatesOpenLoop => 11,
            Self::FulfillsCommitment => 12,
            Self::AssociatedWith => 13,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionState {
    Active,
    Suppressed,
}

impl RetentionState {
    pub(crate) const fn restrictiveness_rank(self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Suppressed => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Active,
    Dormant,
    Resolved,
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

    #[error("a caller-built episode must state its scene")]
    MissingScene,

    #[error("scene time offset {offset_seconds} seconds must be a whole-minute RFC 3339 offset")]
    InvalidSceneTimeOffset { offset_seconds: i32 },

    #[error("observation episode_id must reference an episode")]
    MissingEpisodeReference,

    #[error("derived memory must cite a source episode or observation, or declare application-given grounding")]
    MissingDerivedSource,

    #[error(transparent)]
    InvalidBelief(#[from] BeliefValidationError),

    #[error("{field} must be in 0.0..=1.0 and finite, got {value}")]
    InvalidScore { field: &'static str, value: f32 },

    #[error("Supersedes links are derived from a memory supersedes list and cannot be authored")]
    AuthoredSupersedesLink,

    #[error("About links between interpreted memories and entities must be derived from the memory subject list")]
    AuthoredBeliefAboutLink,

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
    pub scene: Scene,
    pub ended_at: Option<DateTime<Utc>>,
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
        self.scene.validate_time()?;
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
    pub created_at: DateTime<Utc>,
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
    /// The notions this interpreted memory is about (its subjects).
    pub entity_ids: Vec<MemoryId>,
    #[serde(skip)]
    pub(crate) scope_keys: Vec<ScopeKey>,
    /// The character's commitments about subjects in `entity_ids`.
    pub assertions: Vec<BeliefAssertion>,
    /// Source-free grounding given by the application; requires at least one notion subject.
    pub given_by_application: bool,
    pub salience_score: f32,
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
        let has_sources = !self.derived_from_episode_ids.is_empty()
            || !self.derived_from_observation_ids.is_empty();
        belief::validate_belief(
            &self.entity_ids,
            has_sources,
            self.given_by_application,
            &self.assertions,
        )?;
        if !has_sources && !self.given_by_application {
            return Err(DomainValidationError::MissingDerivedSource);
        }
        if self.supersedes.contains(&self.id) {
            return Err(DomainValidationError::SelfLink {
                object_type: ObjectType::DerivedMemory,
                id: self.id,
            });
        }
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
    pub fn id(&self) -> MemoryId {
        match self {
            Self::Episode(object) => object.id,
            Self::Observation(object) => object.id,
            Self::Entity(object) => object.id,
            Self::MemoryThread(object) => object.id,
            Self::DerivedMemory(object) => object.id,
            Self::MemoryLink(object) => object.id,
        }
    }

    pub fn object_type(&self) -> ObjectType {
        match self {
            Self::Episode(object) => object.object_type,
            Self::Observation(object) => object.object_type,
            Self::Entity(object) => object.object_type,
            Self::MemoryThread(object) => object.object_type,
            Self::DerivedMemory(object) => object.object_type,
            Self::MemoryLink(object) => object.object_type,
        }
    }

    pub(crate) fn object_ref(&self) -> MemoryObjectRef {
        MemoryObjectRef::new(self.object_type(), self.id())
    }

    pub(crate) fn stable_order_key(&self) -> (MemoryId, u8) {
        (self.id(), self.object_type().stable_rank())
    }

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

#[cfg(test)]
mod token_tests {
    use super::{ObjectType, VectorSurface};

    #[test]
    fn persisted_object_and_surface_tokens_round_trip() {
        for object_type in [
            ObjectType::Episode,
            ObjectType::Observation,
            ObjectType::Entity,
            ObjectType::MemoryThread,
            ObjectType::DerivedMemory,
            ObjectType::MemoryLink,
        ] {
            assert_eq!(object_type.to_string().parse(), Ok(object_type));
        }
        for surface in [
            VectorSurface::Summary,
            VectorSurface::Text,
            VectorSurface::DerivedText,
            VectorSurface::SceneSetting,
            VectorSurface::SceneParticipants,
            VectorSurface::Query,
        ] {
            assert_eq!(surface.to_string().parse(), Ok(surface));
        }
    }
}
