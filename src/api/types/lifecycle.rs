use crate::domain::{
    DerivedType, LifecycleDtoValidationError, MemoryId, MemoryObjectRef, ObjectType,
    RetentionState, Stability, ThreadStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "object_type", content = "id", rename_all = "snake_case")]
pub enum LifecycleTargetRef {
    DerivedMemory(MemoryId),
    Episode(MemoryId),
    Observation(MemoryId),
    MemoryThread(MemoryId),
}

impl LifecycleTargetRef {
    pub const fn derived_memory(id: MemoryId) -> Self {
        Self::DerivedMemory(id)
    }

    pub const fn episode(id: MemoryId) -> Self {
        Self::Episode(id)
    }

    pub const fn observation(id: MemoryId) -> Self {
        Self::Observation(id)
    }

    pub const fn memory_thread(id: MemoryId) -> Self {
        Self::MemoryThread(id)
    }

    pub const fn object_type(self) -> ObjectType {
        match self {
            Self::DerivedMemory(_) => ObjectType::DerivedMemory,
            Self::Episode(_) => ObjectType::Episode,
            Self::Observation(_) => ObjectType::Observation,
            Self::MemoryThread(_) => ObjectType::MemoryThread,
        }
    }

    pub const fn id(self) -> MemoryId {
        match self {
            Self::DerivedMemory(id)
            | Self::Episode(id)
            | Self::Observation(id)
            | Self::MemoryThread(id) => id,
        }
    }

    pub const fn as_memory_object_ref(self) -> MemoryObjectRef {
        MemoryObjectRef::new(self.object_type(), self.id())
    }
}

impl TryFrom<MemoryObjectRef> for LifecycleTargetRef {
    type Error = LifecycleDtoValidationError;

    fn try_from(value: MemoryObjectRef) -> Result<Self, Self::Error> {
        match value.object_type {
            ObjectType::DerivedMemory => Ok(Self::DerivedMemory(value.id)),
            ObjectType::Episode => Ok(Self::Episode(value.id)),
            ObjectType::Observation => Ok(Self::Observation(value.id)),
            ObjectType::MemoryThread => Ok(Self::MemoryThread(value.id)),
            unsupported => Err(LifecycleDtoValidationError::UnsupportedLifecycleTarget(
                unsupported,
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalSourceReference {
    pub source_ref: Option<String>,
    pub raw_ref: Option<String>,
}

impl ExternalSourceReference {
    pub fn source(source_ref: impl Into<String>) -> Self {
        Self {
            source_ref: Some(source_ref.into()),
            raw_ref: None,
        }
    }

    pub fn raw(raw_ref: impl Into<String>) -> Self {
        Self {
            source_ref: None,
            raw_ref: Some(raw_ref.into()),
        }
    }

    pub fn has_reference(&self) -> bool {
        self.source_ref
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
            || self
                .raw_ref
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceProvenanceReference {
    pub episode_ids: Vec<MemoryId>,
    pub observation_ids: Vec<MemoryId>,
    pub external_refs: Vec<ExternalSourceReference>,
}

impl SourceProvenanceReference {
    pub fn episode(episode_id: MemoryId) -> Self {
        Self {
            episode_ids: vec![episode_id],
            observation_ids: Vec::new(),
            external_refs: Vec::new(),
        }
    }

    pub fn observation(observation_id: MemoryId) -> Self {
        Self {
            episode_ids: Vec::new(),
            observation_ids: vec![observation_id],
            external_refs: Vec::new(),
        }
    }

    pub fn with_external_ref(mut self, external_ref: ExternalSourceReference) -> Self {
        self.external_refs.push(external_ref);
        self
    }

    pub fn has_reference(&self) -> bool {
        !self.episode_ids.is_empty()
            || !self.observation_ids.is_empty()
            || self
                .external_refs
                .iter()
                .any(ExternalSourceReference::has_reference)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "object_type", rename_all = "snake_case")]
pub enum SourceObjectCorrectionTarget {
    Episode {
        id: MemoryId,
        original_raw_ref: Option<String>,
        original_source_ref: Option<String>,
    },
    Observation {
        id: MemoryId,
        original_raw_ref: Option<String>,
        original_source_ref: Option<String>,
    },
}

impl SourceObjectCorrectionTarget {
    pub const fn id(&self) -> MemoryId {
        match self {
            Self::Episode { id, .. } | Self::Observation { id, .. } => *id,
        }
    }

    pub const fn object_type(&self) -> ObjectType {
        match self {
            Self::Episode { .. } => ObjectType::Episode,
            Self::Observation { .. } => ObjectType::Observation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CorrectionTarget {
    DerivedMemory {
        id: MemoryId,
    },
    SourceObject {
        target: SourceObjectCorrectionTarget,
    },
}

impl CorrectionTarget {
    pub const fn derived_memory(id: MemoryId) -> Self {
        Self::DerivedMemory { id }
    }

    pub fn source_object(target: SourceObjectCorrectionTarget) -> Self {
        Self::SourceObject { target }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplacementDerivedMemoryDraft {
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
    pub supersedes: Vec<MemoryId>,
    pub original_source_provenance: SourceProvenanceReference,
    pub correction_origin_provenance: SourceProvenanceReference,
}

impl ReplacementDerivedMemoryDraft {
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
            supersedes: Vec::new(),
            original_source_provenance: SourceProvenanceReference {
                episode_ids: Vec::new(),
                observation_ids: Vec::new(),
                external_refs: Vec::new(),
            },
            correction_origin_provenance: SourceProvenanceReference {
                episode_ids: Vec::new(),
                observation_ids: Vec::new(),
                external_refs: Vec::new(),
            },
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

    pub fn with_superseded_memory(mut self, superseded_memory_id: MemoryId) -> Self {
        self.supersedes.push(superseded_memory_id);
        self
    }

    pub fn validate(&self) -> Result<(), LifecycleDtoValidationError> {
        if self.text.trim().is_empty() {
            return Err(LifecycleDtoValidationError::EmptyReplacementText);
        }

        if self.derived_from_episode_ids.is_empty() && self.derived_from_observation_ids.is_empty()
        {
            return Err(LifecycleDtoValidationError::MissingReplacementSource);
        }

        if !self.correction_origin_provenance.has_reference() {
            return Err(LifecycleDtoValidationError::EmptyCorrectionOrigin);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionLifecyclePolicy {
    pub supersede_replaced_derived_memories: bool,
    pub suppress_superseded_derived_memories: bool,
    pub retain_original_source_objects: bool,
    pub destructive_actions: DeferredDestructiveLifecyclePolicy,
}

impl Default for CorrectionLifecyclePolicy {
    fn default() -> Self {
        Self {
            supersede_replaced_derived_memories: true,
            suppress_superseded_derived_memories: true,
            retain_original_source_objects: true,
            destructive_actions: DeferredDestructiveLifecyclePolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionCascadePolicy {
    pub apply_to_provenanced_derived_memories: bool,
    pub require_original_source_match: bool,
    pub cascade_to_threads: bool,
}

impl Default for CorrectionCascadePolicy {
    fn default() -> Self {
        Self {
            apply_to_provenanced_derived_memories: true,
            require_original_source_match: true,
            cascade_to_threads: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeferredDestructiveLifecyclePolicy {
    pub hard_delete: DeferredLifecycleAction,
    pub redaction: DeferredLifecycleAction,
}

impl Default for DeferredDestructiveLifecyclePolicy {
    fn default() -> Self {
        Self {
            hard_delete: DeferredLifecycleAction::UnsupportedDeferred,
            redaction: DeferredLifecycleAction::UnsupportedDeferred,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeferredLifecycleAction {
    UnsupportedDeferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectMemoryDraft {
    pub targets: Vec<CorrectionTarget>,
    pub replacement_derived_memories: Vec<ReplacementDerivedMemoryDraft>,
    pub superseded_derived_memory_ids: Vec<MemoryId>,
    pub correction_origin: SourceProvenanceReference,
    pub rationale: String,
    pub lifecycle_policy: CorrectionLifecyclePolicy,
    pub cascade_policy: CorrectionCascadePolicy,
    pub include_trace: bool,
}

impl CorrectMemoryDraft {
    pub fn new(target: CorrectionTarget, rationale: impl Into<String>) -> Self {
        Self {
            targets: vec![target],
            replacement_derived_memories: Vec::new(),
            superseded_derived_memory_ids: Vec::new(),
            correction_origin: SourceProvenanceReference {
                episode_ids: Vec::new(),
                observation_ids: Vec::new(),
                external_refs: Vec::new(),
            },
            rationale: rationale.into(),
            lifecycle_policy: CorrectionLifecyclePolicy::default(),
            cascade_policy: CorrectionCascadePolicy::default(),
            include_trace: false,
        }
    }

    pub fn with_replacement(mut self, replacement: ReplacementDerivedMemoryDraft) -> Self {
        self.replacement_derived_memories.push(replacement);
        self
    }

    pub fn with_superseded_derived_memory(mut self, memory_id: MemoryId) -> Self {
        self.superseded_derived_memory_ids.push(memory_id);
        self
    }

    pub fn with_trace(mut self) -> Self {
        self.include_trace = true;
        self
    }

    pub fn validate(&self) -> Result<(), LifecycleDtoValidationError> {
        if self.targets.is_empty() {
            return Err(LifecycleDtoValidationError::MissingCorrectionTarget);
        }

        if self.rationale.trim().is_empty() {
            return Err(LifecycleDtoValidationError::EmptyRationale);
        }

        if !self.correction_origin.has_reference() {
            return Err(LifecycleDtoValidationError::EmptyCorrectionOrigin);
        }

        for replacement in &self.replacement_derived_memories {
            replacement.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppressionPolicy {
    pub suppress_target: bool,
    pub suppress_derived_from_target: bool,
    pub preserve_original_raw_refs: bool,
}

impl Default for SuppressionPolicy {
    fn default() -> Self {
        Self {
            suppress_target: true,
            suppress_derived_from_target: true,
            preserve_original_raw_refs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchivePolicy {
    pub archive_thread: bool,
    pub archive_thread_derived_memories: bool,
    pub preserve_original_raw_refs: bool,
}

impl Default for ArchivePolicy {
    fn default() -> Self {
        Self {
            archive_thread: true,
            archive_thread_derived_memories: false,
            preserve_original_raw_refs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForgetCascadePolicy {
    pub apply_to_derived_from_target: bool,
    pub apply_to_thread_members: bool,
}

impl Default for ForgetCascadePolicy {
    fn default() -> Self {
        Self {
            apply_to_derived_from_target: true,
            apply_to_thread_members: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ForgetLifecyclePolicy {
    pub suppression: SuppressionPolicy,
    pub archive: ArchivePolicy,
    pub destructive_actions: DeferredDestructiveLifecyclePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForgetMemoryDraft {
    pub targets: Vec<LifecycleTargetRef>,
    pub rationale: String,
    pub lifecycle_policy: ForgetLifecyclePolicy,
    pub cascade_policy: ForgetCascadePolicy,
    pub target_retention_state: RetentionState,
    pub target_thread_status: Option<ThreadStatus>,
    pub include_trace: bool,
}

impl ForgetMemoryDraft {
    pub fn suppress(target: LifecycleTargetRef, rationale: impl Into<String>) -> Self {
        Self {
            targets: vec![target],
            rationale: rationale.into(),
            lifecycle_policy: ForgetLifecyclePolicy::default(),
            cascade_policy: ForgetCascadePolicy::default(),
            target_retention_state: RetentionState::Suppressed,
            target_thread_status: None,
            include_trace: false,
        }
    }

    pub fn archive_thread(thread_id: MemoryId, rationale: impl Into<String>) -> Self {
        Self {
            targets: vec![LifecycleTargetRef::MemoryThread(thread_id)],
            rationale: rationale.into(),
            lifecycle_policy: ForgetLifecyclePolicy::default(),
            cascade_policy: ForgetCascadePolicy::default(),
            target_retention_state: RetentionState::Archived,
            target_thread_status: Some(ThreadStatus::Archived),
            include_trace: false,
        }
    }

    pub fn with_trace(mut self) -> Self {
        self.include_trace = true;
        self
    }

    pub fn validate(&self) -> Result<(), LifecycleDtoValidationError> {
        if self.targets.is_empty() {
            return Err(LifecycleDtoValidationError::MissingForgetTarget);
        }

        if self.rationale.trim().is_empty() {
            return Err(LifecycleDtoValidationError::EmptyRationale);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupersededByEvidence {
    pub superseded_memory_id: MemoryId,
    pub superseded_by_memory_id: MemoryId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleMutationTrace {
    pub requested_targets: Vec<LifecycleTargetRef>,
    pub superseded_by: Vec<SupersededByEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleMutationOutcome {
    pub graph_mutated_object_ids: Vec<MemoryObjectRef>,
    pub graph_mutated_link_ids: Vec<MemoryId>,
    pub vector_maintained_object_ids: Vec<MemoryObjectRef>,
    pub vector_maintenance_failure: Option<VectorMaintenanceFailure>,
    pub trace: Option<LifecycleMutationTrace>,
    pub diagnostics: LifecycleMutationDiagnostics,
}

impl LifecycleMutationOutcome {
    pub fn empty() -> Self {
        Self {
            graph_mutated_object_ids: Vec::new(),
            graph_mutated_link_ids: Vec::new(),
            vector_maintained_object_ids: Vec::new(),
            vector_maintenance_failure: None,
            trace: None,
            diagnostics: LifecycleMutationDiagnostics::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleMutationDiagnostics {
    pub warnings: Vec<LifecycleMutationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleMutationWarning {
    pub reason: LifecycleMutationWarningReason,
    pub affected_memory_ids: Vec<MemoryId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleMutationWarningReason {
    CascadeSuppressesCurrentReplacement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorMaintenanceFailure {
    pub unmaintained_object_ids: Vec<MemoryObjectRef>,
    pub error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    fn memory_id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn episode_id() -> MemoryId {
        memory_id("550e8400-e29b-41d4-a716-446655441000")
    }

    fn observation_id() -> MemoryId {
        memory_id("550e8400-e29b-41d4-a716-446655441001")
    }

    fn old_memory_id() -> MemoryId {
        memory_id("550e8400-e29b-41d4-a716-446655441002")
    }

    fn new_memory_id() -> MemoryId {
        memory_id("550e8400-e29b-41d4-a716-446655441003")
    }

    fn correction_origin() -> SourceProvenanceReference {
        SourceProvenanceReference::observation(observation_id())
            .with_external_ref(ExternalSourceReference::raw("raw://corrections/1#message"))
    }

    fn replacement() -> ReplacementDerivedMemoryDraft {
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "The corrected derived memory text.",
        )
        .with_source_episode(episode_id())
        .with_source_observation(observation_id())
        .with_superseded_memory(old_memory_id());
        replacement.id = Some(new_memory_id());
        replacement.original_source_provenance = SourceProvenanceReference::episode(episode_id())
            .with_external_ref(ExternalSourceReference::raw("raw://original/episode"));
        replacement.correction_origin_provenance = correction_origin();
        replacement
    }

    fn correction_draft() -> CorrectMemoryDraft {
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(old_memory_id()),
            "User corrected the earlier derived memory.",
        )
        .with_replacement(replacement())
        .with_superseded_derived_memory(old_memory_id())
        .with_trace();
        draft.correction_origin = correction_origin();
        draft
    }

    #[test]
    fn correction_and_forget_dtos_round_trip_through_serde() {
        let correction = correction_draft();
        let forget = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::observation(observation_id()),
            "Hide this observation from recall.",
        )
        .with_trace();

        let correction_json = serde_json::to_string(&correction).unwrap();
        let forget_json = serde_json::to_string(&forget).unwrap();

        assert_eq!(
            serde_json::from_str::<CorrectMemoryDraft>(&correction_json).unwrap(),
            correction
        );
        assert_eq!(
            serde_json::from_str::<ForgetMemoryDraft>(&forget_json).unwrap(),
            forget
        );
    }

    #[test]
    fn lifecycle_defaults_are_non_destructive_and_trace_is_opt_in() {
        let correction = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(old_memory_id()),
            "Correct stale memory.",
        );
        let forget = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(old_memory_id()),
            "Suppress stale memory.",
        );

        assert!(
            correction
                .lifecycle_policy
                .supersede_replaced_derived_memories
        );
        assert!(correction.lifecycle_policy.retain_original_source_objects);
        assert_eq!(
            correction.lifecycle_policy.destructive_actions.hard_delete,
            DeferredLifecycleAction::UnsupportedDeferred
        );
        assert_eq!(
            forget.lifecycle_policy.destructive_actions.redaction,
            DeferredLifecycleAction::UnsupportedDeferred
        );
        assert!(!correction.include_trace);
        assert!(!forget.include_trace);
    }

    #[test]
    fn validation_rejects_empty_correction_origin_and_replacement_sources() {
        let mut correction = correction_draft();
        correction.correction_origin = SourceProvenanceReference {
            episode_ids: Vec::new(),
            observation_ids: Vec::new(),
            external_refs: Vec::new(),
        };
        assert_eq!(
            correction.validate(),
            Err(LifecycleDtoValidationError::EmptyCorrectionOrigin)
        );

        let mut replacement = replacement();
        replacement.derived_from_episode_ids.clear();
        replacement.derived_from_observation_ids.clear();
        assert_eq!(
            replacement.validate(),
            Err(LifecycleDtoValidationError::MissingReplacementSource)
        );
    }

    #[test]
    fn validation_rejects_whitespace_only_external_refs() {
        assert!(!ExternalSourceReference::raw("   ").has_reference());
        assert!(!ExternalSourceReference::source("\t\n").has_reference());

        let mut correction = correction_draft();
        correction.correction_origin = SourceProvenanceReference {
            episode_ids: Vec::new(),
            observation_ids: Vec::new(),
            external_refs: vec![ExternalSourceReference::raw("  ")],
        };
        assert_eq!(
            correction.validate(),
            Err(LifecycleDtoValidationError::EmptyCorrectionOrigin)
        );
    }

    #[test]
    fn validation_requires_targets_and_rationale() {
        let mut correction = correction_draft();
        correction.targets.clear();
        assert_eq!(
            correction.validate(),
            Err(LifecycleDtoValidationError::MissingCorrectionTarget)
        );

        let mut forget =
            ForgetMemoryDraft::suppress(LifecycleTargetRef::episode(episode_id()), "  \n\t  ");
        assert_eq!(
            forget.validate(),
            Err(LifecycleDtoValidationError::EmptyRationale)
        );

        forget.rationale = "Suppress episode.".to_owned();
        forget.targets.clear();
        assert_eq!(
            forget.validate(),
            Err(LifecycleDtoValidationError::MissingForgetTarget)
        );
    }

    #[test]
    fn validation_errors_have_actionable_display_messages() {
        assert_eq!(
            LifecycleDtoValidationError::MissingForgetTarget.to_string(),
            "forget requires at least one target"
        );
        assert_eq!(
            LifecycleDtoValidationError::UnsupportedLifecycleTarget(ObjectType::Entity).to_string(),
            "unsupported lifecycle target: Entity"
        );
    }

    #[test]
    fn typed_target_boundaries_accept_supported_lifecycle_objects_only() {
        for object_type in [
            ObjectType::DerivedMemory,
            ObjectType::Episode,
            ObjectType::Observation,
            ObjectType::MemoryThread,
        ] {
            let target =
                LifecycleTargetRef::try_from(MemoryObjectRef::new(object_type, episode_id()));
            assert!(target.is_ok());
        }

        assert_eq!(
            LifecycleTargetRef::try_from(MemoryObjectRef::new(ObjectType::Entity, episode_id())),
            Err(LifecycleDtoValidationError::UnsupportedLifecycleTarget(
                ObjectType::Entity
            ))
        );
        assert_eq!(
            LifecycleTargetRef::try_from(MemoryObjectRef::new(
                ObjectType::MemoryLink,
                episode_id()
            )),
            Err(LifecycleDtoValidationError::UnsupportedLifecycleTarget(
                ObjectType::MemoryLink
            ))
        );
    }

    #[test]
    fn correction_semantics_supersede_without_requesting_destruction() {
        let draft = correction_draft();
        let replacement = draft.replacement_derived_memories.first().unwrap();

        assert_eq!(replacement.supersedes, vec![old_memory_id()]);
        assert_eq!(draft.superseded_derived_memory_ids, vec![old_memory_id()]);
        assert!(draft.lifecycle_policy.suppress_superseded_derived_memories);
        assert_eq!(
            draft.lifecycle_policy.destructive_actions,
            DeferredDestructiveLifecyclePolicy::default()
        );
        assert_eq!(draft.validate(), Ok(()));
    }

    #[test]
    fn source_target_cascade_defaults_follow_provenanced_derived_memories() {
        let draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: episode_id(),
                original_raw_ref: Some("raw://original/episode".to_owned()),
                original_source_ref: Some("conversation://original".to_owned()),
            }),
            "Correct source episode summary and derived claims.",
        );

        assert!(draft.cascade_policy.apply_to_provenanced_derived_memories);
        assert!(draft.cascade_policy.require_original_source_match);
        assert!(!draft.cascade_policy.cascade_to_threads);
    }

    #[test]
    fn original_source_and_correction_origin_provenance_are_distinct_refs() {
        let replacement = replacement();
        let serialized = serde_json::to_string(&replacement).unwrap();

        assert_eq!(
            replacement.original_source_provenance.external_refs[0]
                .raw_ref
                .as_deref(),
            Some("raw://original/episode")
        );
        assert_eq!(
            replacement.correction_origin_provenance.external_refs[0]
                .raw_ref
                .as_deref(),
            Some("raw://corrections/1#message")
        );
        assert!(!serialized.contains("verbatim transcript payload"));
    }

    #[test]
    fn suppression_and_archive_defaults_match_supported_lifecycle_boundary() {
        let suppression = SuppressionPolicy::default();
        let archive = ArchivePolicy::default();
        let thread_forget =
            ForgetMemoryDraft::archive_thread(episode_id(), "Archive finished thread.");

        assert!(suppression.suppress_target);
        assert!(suppression.suppress_derived_from_target);
        assert!(suppression.preserve_original_raw_refs);
        assert!(archive.archive_thread);
        assert!(!archive.archive_thread_derived_memories);
        assert_eq!(
            thread_forget.target_retention_state,
            RetentionState::Archived
        );
        assert_eq!(
            thread_forget.target_thread_status,
            Some(ThreadStatus::Archived)
        );
    }

    #[test]
    fn hard_delete_and_redaction_are_deferred_not_selectable_behaviors() {
        let policy = DeferredDestructiveLifecyclePolicy::default();
        let serialized = serde_json::to_string(&policy).unwrap();

        assert_eq!(
            policy.hard_delete,
            DeferredLifecycleAction::UnsupportedDeferred
        );
        assert_eq!(
            policy.redaction,
            DeferredLifecycleAction::UnsupportedDeferred
        );
        assert!(serialized.contains("unsupported_deferred"));
    }

    #[test]
    fn outcome_reports_graph_vector_and_partial_vector_failures() {
        let outcome = LifecycleMutationOutcome {
            graph_mutated_object_ids: vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                new_memory_id(),
            )],
            graph_mutated_link_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655441004")],
            vector_maintained_object_ids: vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                new_memory_id(),
            )],
            vector_maintenance_failure: Some(VectorMaintenanceFailure {
                unmaintained_object_ids: vec![MemoryObjectRef::new(
                    ObjectType::DerivedMemory,
                    old_memory_id(),
                )],
                error_message: "vector maintenance timed out after graph mutation".to_owned(),
            }),
            trace: Some(LifecycleMutationTrace {
                requested_targets: vec![LifecycleTargetRef::derived_memory(old_memory_id())],
                superseded_by: vec![SupersededByEvidence {
                    superseded_memory_id: old_memory_id(),
                    superseded_by_memory_id: new_memory_id(),
                }],
            }),
            diagnostics: LifecycleMutationDiagnostics {
                warnings: vec![LifecycleMutationWarning {
                    reason: LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement,
                    affected_memory_ids: vec![new_memory_id()],
                }],
            },
        };

        let serialized = serde_json::to_string(&outcome).unwrap();
        let round_tripped: LifecycleMutationOutcome = serde_json::from_str(&serialized).unwrap();

        assert_eq!(round_tripped, outcome);
        assert!(serialized.contains("cascade-suppresses-current-replacement"));
        assert_eq!(
            round_tripped.trace.unwrap().superseded_by[0],
            SupersededByEvidence {
                superseded_memory_id: old_memory_id(),
                superseded_by_memory_id: new_memory_id(),
            }
        );
    }
}
