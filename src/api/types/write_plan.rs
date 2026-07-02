use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::domain::{MemoryId, RelationType};
use super::draft::{
    DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryLinkDraft, MemoryThreadDraft,
    ObservationDraft, VectorIndexingFailure,
};
use super::lifecycle::ExternalSourceReference;
use super::retrieval::MemoryObjectRef;

pub mod helpers;
pub use helpers::{PreparedCandidateRefs, RememberPlanDefaults};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RememberInput {
    pub content: String,
    pub entity_ids: Vec<MemoryId>,
    pub thread_ids: Vec<MemoryId>,
    pub scope_ids: Vec<String>,
    pub participant_entity_ids: Vec<MemoryId>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub raw_refs: Vec<String>,
    pub source_spans: Vec<SourceSpan>,
    pub episode_drafts: Vec<EpisodeDraft>,
    pub observation_drafts: Vec<ObservationDraft>,
    pub entity_drafts: Vec<EntityDraft>,
    pub memory_thread_drafts: Vec<MemoryThreadDraft>,
    pub derived_memory_drafts: Vec<DerivedMemoryDraft>,
    pub memory_link_drafts: Vec<MemoryLinkDraft>,
}

impl RememberInput {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            entity_ids: Vec::new(),
            thread_ids: Vec::new(),
            scope_ids: Vec::new(),
            participant_entity_ids: Vec::new(),
            started_at: None,
            ended_at: None,
            raw_refs: Vec::new(),
            source_spans: Vec::new(),
            episode_drafts: Vec::new(),
            observation_drafts: Vec::new(),
            entity_drafts: Vec::new(),
            memory_thread_drafts: Vec::new(),
            derived_memory_drafts: Vec::new(),
            memory_link_drafts: Vec::new(),
        }
    }

    pub fn with_entity_id(mut self, entity_id: MemoryId) -> Self {
        self.entity_ids.push(entity_id);
        self
    }

    pub fn with_thread_id(mut self, thread_id: MemoryId) -> Self {
        self.thread_ids.push(thread_id);
        self
    }

    pub fn with_scope_id(mut self, scope_id: impl Into<String>) -> Self {
        self.scope_ids.push(scope_id.into());
        self
    }

    pub fn with_participant_entity_id(mut self, participant_entity_id: MemoryId) -> Self {
        self.participant_entity_ids.push(participant_entity_id);
        self
    }

    pub fn with_started_at(mut self, started_at: DateTime<Utc>) -> Self {
        self.started_at = Some(started_at);
        self
    }

    pub fn with_ended_at(mut self, ended_at: DateTime<Utc>) -> Self {
        self.ended_at = Some(ended_at);
        self
    }

    pub fn with_raw_ref(mut self, raw_ref: impl Into<String>) -> Self {
        self.raw_refs.push(raw_ref.into());
        self
    }

    pub fn with_source_span(mut self, source_span: SourceSpan) -> Self {
        self.source_spans.push(source_span);
        self
    }

    pub fn with_episode(mut self, episode: EpisodeDraft) -> Self {
        self.episode_drafts.push(episode);
        self
    }

    pub fn with_observation(mut self, observation: ObservationDraft) -> Self {
        self.observation_drafts.push(observation);
        self
    }

    pub fn with_entity(mut self, entity: EntityDraft) -> Self {
        self.entity_drafts.push(entity);
        self
    }

    pub fn with_memory_thread(mut self, memory_thread: MemoryThreadDraft) -> Self {
        self.memory_thread_drafts.push(memory_thread);
        self
    }

    pub fn with_derived_memory(mut self, derived_memory: DerivedMemoryDraft) -> Self {
        self.derived_memory_drafts.push(derived_memory);
        self
    }

    pub fn with_memory_link(mut self, memory_link: MemoryLinkDraft) -> Self {
        self.memory_link_drafts.push(memory_link);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "candidate_type",
    content = "candidate",
    rename_all = "snake_case"
)]
pub enum MemoryCandidate {
    Episode(EpisodeCandidate),
    Observation(ObservationCandidate),
    Entity(EntityCandidate),
    MemoryThread(MemoryThreadCandidate),
    DerivedMemory(DerivedMemoryCandidate),
    MemoryLink(MemoryLinkCandidate),
    VectorIndex(VectorIndexCandidate),
    StatsUpdate(StatsUpdateCandidate),
}

impl MemoryCandidate {
    pub const fn kind(&self) -> MemoryCandidateKind {
        match self {
            Self::Episode(_) => MemoryCandidateKind::Episode,
            Self::Observation(_) => MemoryCandidateKind::Observation,
            Self::Entity(_) => MemoryCandidateKind::Entity,
            Self::MemoryThread(_) => MemoryCandidateKind::MemoryThread,
            Self::DerivedMemory(_) => MemoryCandidateKind::DerivedMemory,
            Self::MemoryLink(_) => MemoryCandidateKind::MemoryLink,
            Self::VectorIndex(_) => MemoryCandidateKind::VectorIndex,
            Self::StatsUpdate(_) => MemoryCandidateKind::StatsUpdate,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCandidateKind {
    Episode,
    Observation,
    Entity,
    MemoryThread,
    DerivedMemory,
    MemoryLink,
    VectorIndex,
    StatsUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeCandidate {
    pub draft: EpisodeDraft,
    pub provenance: CandidateProvenance,
}

impl EpisodeCandidate {
    pub fn new(draft: EpisodeDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationCandidate {
    pub draft: ObservationDraft,
    pub provenance: CandidateProvenance,
}

impl ObservationCandidate {
    pub fn new(draft: ObservationDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityCandidate {
    pub draft: EntityDraft,
    pub provenance: CandidateProvenance,
}

impl EntityCandidate {
    pub fn new(draft: EntityDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryThreadCandidate {
    pub draft: MemoryThreadDraft,
    pub provenance: CandidateProvenance,
}

impl MemoryThreadCandidate {
    pub fn new(draft: MemoryThreadDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivedMemoryCandidate {
    pub draft: DerivedMemoryDraft,
    pub provenance: CandidateProvenance,
}

impl DerivedMemoryCandidate {
    pub fn new(draft: DerivedMemoryDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryLinkCandidate {
    pub draft: MemoryLinkDraft,
    pub provenance: CandidateProvenance,
}

impl MemoryLinkCandidate {
    pub fn new(draft: MemoryLinkDraft, provenance: CandidateProvenance) -> Self {
        Self { draft, provenance }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorIndexCandidate {
    pub target: MemoryObjectRef,
    pub embedding_text: String,
    pub provenance: CandidateProvenance,
}

impl VectorIndexCandidate {
    pub fn new(
        target: MemoryObjectRef,
        embedding_text: impl Into<String>,
        provenance: CandidateProvenance,
    ) -> Self {
        Self {
            target,
            embedding_text: embedding_text.into(),
            provenance,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsUpdateCandidate {
    pub subject: MemoryObjectRef,
    pub relation: Option<RelationType>,
    pub object: Option<MemoryObjectRef>,
    pub provenance: CandidateProvenance,
}

impl StatsUpdateCandidate {
    pub fn new(subject: MemoryObjectRef, provenance: CandidateProvenance) -> Self {
        Self {
            subject,
            relation: None,
            object: None,
            provenance,
        }
    }

    pub fn with_relation(mut self, relation: RelationType, object: MemoryObjectRef) -> Self {
        self.relation = Some(relation);
        self.object = Some(object);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RememberWritePlan {
    pub operation_id: MemoryId,
    pub idempotency_key: String,
    pub source_input_ref: Option<ExternalSourceReference>,
    pub candidates: Vec<MemoryCandidate>,
    pub validations: Vec<CandidateValidation>,
    pub diagnostics: RememberDiagnostics,
}

impl RememberWritePlan {
    pub fn new(operation_id: MemoryId, idempotency_key: impl Into<String>) -> Self {
        Self {
            operation_id,
            idempotency_key: idempotency_key.into(),
            source_input_ref: None,
            candidates: Vec::new(),
            validations: Vec::new(),
            diagnostics: RememberDiagnostics::default(),
        }
    }

    pub fn with_source_input_ref(mut self, source_input_ref: ExternalSourceReference) -> Self {
        self.source_input_ref = Some(source_input_ref);
        self
    }

    pub fn with_candidate(mut self, candidate: MemoryCandidate) -> Self {
        self.candidates.push(candidate);
        self
    }

    pub fn with_validation(mut self, validation: CandidateValidation) -> Self {
        self.validations.push(validation);
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: RememberDiagnostics) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateValidation {
    pub candidate_index: usize,
    pub candidate_kind: MemoryCandidateKind,
    pub status: CandidateValidationStatus,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CandidateValidation {
    pub fn valid(candidate_index: usize, candidate_kind: MemoryCandidateKind) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Valid,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(
        candidate_index: usize,
        candidate_kind: MemoryCandidateKind,
        error: impl Into<String>,
    ) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Invalid,
            errors: vec![error.into()],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateValidationStatus {
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateProvenance {
    pub producer_kind: CandidateProducerKind,
    pub rationale: CandidateRationale,
    pub source: SourceProvenance,
}

impl CandidateProvenance {
    pub fn new(producer_kind: CandidateProducerKind) -> Self {
        Self {
            producer_kind,
            rationale: CandidateRationale::Unavailable,
            source: SourceProvenance::default(),
        }
    }

    pub fn caller(rationale: impl Into<String>) -> Self {
        Self::new(CandidateProducerKind::Caller)
            .with_rationale(CandidateRationale::provided_by_caller(rationale))
    }

    pub fn processor(producer_kind: CandidateProducerKind, rationale: impl Into<String>) -> Self {
        Self::new(producer_kind)
            .with_rationale(CandidateRationale::provided_by_processor(rationale))
    }

    pub fn inferred_by_processor(
        producer_kind: CandidateProducerKind,
        rationale: impl Into<String>,
    ) -> Self {
        Self::new(producer_kind)
            .with_rationale(CandidateRationale::inferred_by_processor(rationale))
    }

    pub fn unavailable(producer_kind: CandidateProducerKind) -> Self {
        Self::new(producer_kind)
    }

    pub fn with_rationale(mut self, rationale: CandidateRationale) -> Self {
        self.rationale = rationale;
        self
    }

    pub fn with_source_span(mut self, source_span: SourceSpan) -> Self {
        self.source.source_spans.push(source_span);
        self
    }

    pub fn with_source_episode(mut self, episode_id: MemoryId) -> Self {
        self.source.episode_ids.push(episode_id);
        self
    }

    pub fn with_source_observation(mut self, observation_id: MemoryId) -> Self {
        self.source.observation_ids.push(observation_id);
        self
    }

    pub fn with_external_ref(mut self, external_ref: ExternalSourceReference) -> Self {
        self.source.external_refs.push(external_ref);
        self
    }

    pub const fn rationale_origin(&self) -> RationaleOrigin {
        self.rationale.origin()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateProducerKind {
    Caller,
    DeterministicHelper,
    RuleProcessor,
    ModelProcessor,
    ImportTool,
    System,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "origin", content = "text", rename_all = "snake_case")]
pub enum CandidateRationale {
    ProvidedByCaller(String),
    ProvidedByProcessor(String),
    InferredByProcessor(String),
    Unavailable,
}

impl CandidateRationale {
    pub fn provided_by_caller(text: impl Into<String>) -> Self {
        Self::ProvidedByCaller(text.into())
    }

    pub fn provided_by_processor(text: impl Into<String>) -> Self {
        Self::ProvidedByProcessor(text.into())
    }

    pub fn inferred_by_processor(text: impl Into<String>) -> Self {
        Self::InferredByProcessor(text.into())
    }

    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    pub const fn origin(&self) -> RationaleOrigin {
        match self {
            Self::ProvidedByCaller(_) => RationaleOrigin::ProvidedByCaller,
            Self::ProvidedByProcessor(_) => RationaleOrigin::ProvidedByProcessor,
            Self::InferredByProcessor(_) => RationaleOrigin::InferredByProcessor,
            Self::Unavailable => RationaleOrigin::Unavailable,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::ProvidedByCaller(text)
            | Self::ProvidedByProcessor(text)
            | Self::InferredByProcessor(text) => Some(text),
            Self::Unavailable => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RationaleOrigin {
    ProvidedByCaller,
    ProvidedByProcessor,
    InferredByProcessor,
    Unavailable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceProvenance {
    pub episode_ids: Vec<MemoryId>,
    pub observation_ids: Vec<MemoryId>,
    pub external_refs: Vec<ExternalSourceReference>,
    pub source_spans: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub source_ref: Option<String>,
    pub raw_ref: Option<String>,
    pub message_id: Option<String>,
    pub transcript_segment_id: Option<String>,
    pub turn_range: Option<SourceSpanRange<u32>>,
    pub char_range: Option<SourceSpanRange<u32>>,
    pub byte_range: Option<SourceSpanRange<u32>>,
    pub timestamp_range: Option<SourceSpanRange<DateTime<Utc>>>,
}

impl SourceSpan {
    pub fn raw(raw_ref: impl Into<String>) -> Self {
        Self {
            source_ref: None,
            raw_ref: Some(raw_ref.into()),
            message_id: None,
            transcript_segment_id: None,
            turn_range: None,
            char_range: None,
            byte_range: None,
            timestamp_range: None,
        }
    }

    pub fn source(source_ref: impl Into<String>) -> Self {
        Self {
            source_ref: Some(source_ref.into()),
            raw_ref: None,
            message_id: None,
            transcript_segment_id: None,
            turn_range: None,
            char_range: None,
            byte_range: None,
            timestamp_range: None,
        }
    }

    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    pub fn with_transcript_segment_id(mut self, transcript_segment_id: impl Into<String>) -> Self {
        self.transcript_segment_id = Some(transcript_segment_id.into());
        self
    }

    pub fn with_turn_range(mut self, start: u32, end: u32) -> Self {
        self.turn_range = Some(SourceSpanRange::new(start, end));
        self
    }

    pub fn with_char_range(mut self, start: u32, end: u32) -> Self {
        self.char_range = Some(SourceSpanRange::new(start, end));
        self
    }

    pub fn with_byte_range(mut self, start: u32, end: u32) -> Self {
        self.byte_range = Some(SourceSpanRange::new(start, end));
        self
    }

    pub fn with_timestamp_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.timestamp_range = Some(SourceSpanRange::new(start, end));
        self
    }

    pub fn validate(&self) -> Result<(), SourceSpanValidationError> {
        if self
            .source_ref
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceSpanValidationError::EmptySourceRef);
        }
        if self
            .raw_ref
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceSpanValidationError::EmptyRawRef);
        }
        if self
            .message_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceSpanValidationError::EmptyMessageId);
        }
        if self
            .transcript_segment_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SourceSpanValidationError::EmptyTranscriptSegmentId);
        }

        validate_range(
            &self.turn_range,
            SourceSpanValidationError::InvalidTurnRange,
        )?;
        validate_range(
            &self.char_range,
            SourceSpanValidationError::InvalidCharRange,
        )?;
        validate_range(
            &self.byte_range,
            SourceSpanValidationError::InvalidByteRange,
        )?;
        validate_range(
            &self.timestamp_range,
            SourceSpanValidationError::InvalidTimestampRange,
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpanRange<T> {
    pub start: T,
    pub end: T,
}

impl<T> SourceSpanRange<T> {
    pub const fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Error)]
pub enum SourceSpanValidationError {
    #[error("source_ref must not be empty")]
    EmptySourceRef,
    #[error("raw_ref must not be empty")]
    EmptyRawRef,
    #[error("message_id must not be empty")]
    EmptyMessageId,
    #[error("transcript_segment_id must not be empty")]
    EmptyTranscriptSegmentId,
    #[error("turn range start must be less than or equal to end")]
    InvalidTurnRange,
    #[error("character range start must be less than or equal to end")]
    InvalidCharRange,
    #[error("byte range start must be less than or equal to end")]
    InvalidByteRange,
    #[error("timestamp range start must be less than or equal to end")]
    InvalidTimestampRange,
}

fn validate_range<T: Ord>(
    range: &Option<SourceSpanRange<T>>,
    error: SourceSpanValidationError,
) -> Result<(), SourceSpanValidationError> {
    if range.as_ref().is_some_and(|range| range.start > range.end) {
        return Err(error);
    }

    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberDiagnostics {
    pub candidate_counts: Vec<CandidateCount>,
    pub validation_failures: Vec<CandidateValidation>,
    pub messages: Vec<RememberDiagnostic>,
    pub repair_needed: Vec<RepairMarker>,
}

impl RememberDiagnostics {
    pub fn with_candidate_count(
        mut self,
        candidate_kind: MemoryCandidateKind,
        count: usize,
    ) -> Self {
        self.candidate_counts.push(CandidateCount {
            candidate_kind,
            count,
        });
        self
    }

    pub fn with_validation_failure(mut self, failure: CandidateValidation) -> Self {
        self.validation_failures.push(failure);
        self
    }

    pub fn with_message(mut self, message: RememberDiagnostic) -> Self {
        self.messages.push(message);
        self
    }

    pub fn with_repair_needed(mut self, repair_needed: RepairMarker) -> Self {
        self.repair_needed.push(repair_needed);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateCount {
    pub candidate_kind: MemoryCandidateKind,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

impl RememberDiagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepairMarker {
    VectorIndex {
        unindexed_object_ids: Vec<MemoryId>,
        error_message: String,
    },
    StatsUpdate {
        object_ids: Vec<MemoryId>,
        error_message: String,
    },
}

impl From<VectorIndexingFailure> for RepairMarker {
    fn from(value: VectorIndexingFailure) -> Self {
        Self::VectorIndex {
            unindexed_object_ids: value.unindexed_object_ids,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsUpdateStatus {
    pub updated_object_ids: Vec<MemoryId>,
    pub failure: Option<StatsUpdateFailure>,
}

impl StatsUpdateStatus {
    pub fn succeeded(updated_object_ids: impl IntoIterator<Item = MemoryId>) -> Self {
        Self {
            updated_object_ids: updated_object_ids.into_iter().collect(),
            failure: None,
        }
    }

    pub fn failed(
        updated_object_ids: impl IntoIterator<Item = MemoryId>,
        failed_object_ids: impl IntoIterator<Item = MemoryId>,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            updated_object_ids: updated_object_ids.into_iter().collect(),
            failure: Some(StatsUpdateFailure {
                failed_object_ids: failed_object_ids.into_iter().collect(),
                error_message: error_message.into(),
            }),
        }
    }
}

impl Default for StatsUpdateStatus {
    fn default() -> Self {
        Self::succeeded([])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsUpdateFailure {
    pub failed_object_ids: Vec<MemoryId>,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrepareOptions {
    pub idempotency_key: Option<String>,
    pub include_vector_index_candidates: bool,
    pub include_stats_update_candidates: bool,
}

impl Default for PrepareOptions {
    fn default() -> Self {
        Self {
            idempotency_key: None,
            include_vector_index_candidates: true,
            include_stats_update_candidates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitOptions {
    pub require_valid_plan: bool,
    pub update_vectors: bool,
    pub update_stats: bool,
}

impl Default for CommitOptions {
    fn default() -> Self {
        Self {
            require_valid_plan: true,
            update_vectors: true,
            update_stats: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RememberOptions {
    pub prepare: PrepareOptions,
    pub commit: CommitOptions,
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
    fn source_span_accepts_ordered_ranges_and_rejects_invalid_ranges() {
        let valid = SourceSpan::raw("raw://conversation/42")
            .with_turn_range(1, 2)
            .with_char_range(10, 20)
            .with_byte_range(11, 30)
            .with_timestamp_range(
                timestamp("2026-07-02T12:00:00Z"),
                timestamp("2026-07-02T12:05:00Z"),
            );
        assert_eq!(valid.validate(), Ok(()));

        let invalid_chars = SourceSpan::source("conversation-42").with_char_range(20, 10);
        assert_eq!(
            invalid_chars.validate(),
            Err(SourceSpanValidationError::InvalidCharRange)
        );

        let invalid_empty_raw = SourceSpan::raw(" ");
        assert_eq!(
            invalid_empty_raw.validate(),
            Err(SourceSpanValidationError::EmptyRawRef)
        );
    }

    #[test]
    fn inferred_rationale_cannot_claim_caller_origin_through_constructors() {
        let provenance = CandidateProvenance::inferred_by_processor(
            CandidateProducerKind::ModelProcessor,
            "candidate was inferred from a transcript segment",
        );

        assert_eq!(
            provenance.rationale_origin(),
            RationaleOrigin::InferredByProcessor
        );
        assert!(matches!(
            provenance.rationale,
            CandidateRationale::InferredByProcessor(_)
        ));
    }

    #[test]
    fn missing_rationale_is_explicitly_representable() {
        let provenance = CandidateProvenance::unavailable(CandidateProducerKind::Unknown);

        assert_eq!(provenance.rationale_origin(), RationaleOrigin::Unavailable);
        assert_eq!(provenance.rationale.text(), None);
    }

    #[test]
    fn write_plan_round_trips_through_serde() {
        let operation_id = memory_id("550e8400-e29b-41d4-a716-446655442001");
        let episode = EpisodeCandidate::new(
            EpisodeDraft::new("Discussed inspectable write planning."),
            CandidateProvenance::caller("caller supplied the episode summary")
                .with_source_span(SourceSpan::raw("raw://conversation/42").with_turn_range(0, 1)),
        );
        let plan = RememberWritePlan::new(operation_id, "remember:42")
            .with_source_input_ref(ExternalSourceReference::raw("raw://conversation/42"))
            .with_candidate(MemoryCandidate::Episode(episode))
            .with_validation(CandidateValidation::valid(0, MemoryCandidateKind::Episode))
            .with_diagnostics(
                RememberDiagnostics::default()
                    .with_candidate_count(MemoryCandidateKind::Episode, 1)
                    .with_message(RememberDiagnostic::new(
                        DiagnosticSeverity::Info,
                        "prepared",
                        "prepared one candidate",
                    )),
            );

        let serialized = serde_json::to_string(&plan).unwrap();
        let deserialized: RememberWritePlan = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, plan);
    }
}
