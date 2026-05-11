use serde::{Deserialize, Serialize};

use super::domain::{
    DerivedMemory, Episode, MemoryId, MemoryThread, ObjectType, Observation, RelationType,
    RetentionState,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalContext {
    pub query_text: String,
    pub current_context: Option<String>,
    pub candidate_limits: RetrievalCandidateLimits,
    pub graph_limits: RetrievalGraphLimits,
    pub section_limits: ContinuitySectionLimits,
    pub lifecycle_policy: RetrievalLifecyclePolicy,
    pub include_trace: bool,
    pub object_type_defaults: Vec<ObjectType>,
}

impl RetrievalContext {
    pub fn new(query_text: impl Into<String>) -> Self {
        Self {
            query_text: query_text.into(),
            ..Self::default()
        }
    }

    pub fn with_current_context(mut self, current_context: impl Into<String>) -> Self {
        self.current_context = Some(current_context.into());
        self
    }

    pub fn with_trace(mut self) -> Self {
        self.include_trace = true;
        self
    }
}

impl Default for RetrievalContext {
    fn default() -> Self {
        Self {
            query_text: String::new(),
            current_context: None,
            candidate_limits: RetrievalCandidateLimits::default(),
            graph_limits: RetrievalGraphLimits::default(),
            section_limits: ContinuitySectionLimits::default(),
            lifecycle_policy: RetrievalLifecyclePolicy::default(),
            include_trace: false,
            object_type_defaults: default_retrieval_object_types(),
        }
    }
}

pub fn default_retrieval_object_types() -> Vec<ObjectType> {
    vec![
        ObjectType::Episode,
        ObjectType::Observation,
        ObjectType::DerivedMemory,
        ObjectType::MemoryThread,
        ObjectType::Entity,
    ]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalCandidateLimits {
    pub max_vector_candidates: usize,
    pub max_graph_roots: usize,
}

impl Default for RetrievalCandidateLimits {
    fn default() -> Self {
        Self {
            max_vector_candidates: 48,
            max_graph_roots: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalGraphLimits {
    pub max_depth: u8,
    pub max_nodes: usize,
    pub max_fanout_per_node: usize,
    pub max_hub_edges: usize,
    pub timeout_ms: Option<u64>,
    pub allow_degraded_results: bool,
    pub allowed_relation_types: Vec<RelationType>,
}

impl Default for RetrievalGraphLimits {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_nodes: 96,
            max_fanout_per_node: 16,
            max_hub_edges: 64,
            timeout_ms: Some(250),
            allow_degraded_results: true,
            allowed_relation_types: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinuitySectionLimits {
    pub active_threads: usize,
    pub relevant_episodes: usize,
    pub salient_observations: usize,
    pub derived_memories: usize,
    pub preferences: usize,
    pub relationship_notes: usize,
    pub open_loops: usize,
    pub commitments: usize,
    pub character_signals: usize,
}

impl Default for ContinuitySectionLimits {
    fn default() -> Self {
        Self {
            active_threads: 6,
            relevant_episodes: 8,
            salient_observations: 16,
            derived_memories: 12,
            preferences: 8,
            relationship_notes: 8,
            open_loops: 8,
            commitments: 8,
            character_signals: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RetrievalLifecyclePolicy {
    pub include_archived: bool,
    pub include_suppressed: bool,
    pub include_deleted: bool,
    pub include_non_current: bool,
    /// Applies to graph-verified supersession evidence reported as `superseded_by`.
    /// A derived memory's local `supersedes` list points to older memories it replaces.
    pub include_superseded: bool,
}

impl RetrievalLifecyclePolicy {
    pub fn allows_retention_state(self, retention_state: RetentionState) -> bool {
        match retention_state {
            RetentionState::Active => true,
            RetentionState::Archived => self.include_archived,
            RetentionState::Suppressed => self.include_suppressed,
            RetentionState::Deleted => self.include_deleted,
        }
    }

    pub fn allows_derived_memory(self, derived_memory: &DerivedMemory) -> bool {
        self.allows_retention_state(derived_memory.retention_state)
            && (derived_memory.is_current || self.include_non_current)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrieveOutcome {
    pub pack: ContinuityContextPack,
    pub rationale: RetrievalRationale,
    pub trace: Option<RetrievalTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContinuityContextPack {
    pub active_threads: Vec<MemoryThread>,
    pub relevant_episodes: Vec<Episode>,
    pub salient_observations: Vec<Observation>,
    pub derived_memories: Vec<IncludedDerivedMemory>,
    pub preferences: Vec<IncludedDerivedMemory>,
    pub relationship_notes: Vec<IncludedDerivedMemory>,
    pub open_loops: Vec<IncludedDerivedMemory>,
    pub commitments: Vec<IncludedDerivedMemory>,
    pub character_signals: Vec<IncludedDerivedMemory>,
}

impl ContinuityContextPack {
    pub fn empty() -> Self {
        Self {
            active_threads: Vec::new(),
            relevant_episodes: Vec::new(),
            salient_observations: Vec::new(),
            derived_memories: Vec::new(),
            preferences: Vec::new(),
            relationship_notes: Vec::new(),
            open_loops: Vec::new(),
            commitments: Vec::new(),
            character_signals: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IncludedDerivedMemory {
    pub memory: DerivedMemory,
    pub source_episode_ids: Vec<MemoryId>,
    pub source_observation_ids: Vec<MemoryId>,
}

impl From<DerivedMemory> for IncludedDerivedMemory {
    fn from(memory: DerivedMemory) -> Self {
        Self {
            source_episode_ids: memory.derived_from_episode_ids.clone(),
            source_observation_ids: memory.derived_from_observation_ids.clone(),
            memory,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalRationale {
    pub summary: String,
    pub vector_candidate_count: usize,
    pub graph_verified_count: usize,
    pub stale_candidate_omission_count: usize,
    pub stale_candidate_omission_reasons: Vec<StaleCandidateOmissionSummary>,
    pub lifecycle_omission_count: usize,
    pub lifecycle_omission_reasons: Vec<LifecycleOmissionSummary>,
    #[serde(default)]
    pub telemetry: RetrievalTelemetry,
}

impl RetrievalRationale {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            vector_candidate_count: 0,
            graph_verified_count: 0,
            stale_candidate_omission_count: 0,
            stale_candidate_omission_reasons: Vec::new(),
            lifecycle_omission_count: 0,
            lifecycle_omission_reasons: Vec::new(),
            telemetry: RetrievalTelemetry::default(),
        }
    }
}

impl Default for RetrievalRationale {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetrievalTelemetry {
    pub configured_candidate_limits: RetrievalCandidateLimits,
    pub configured_graph_limits: RetrievalGraphLimits,
    pub configured_section_limits: ContinuitySectionLimits,
    pub query_embedding_dimension: usize,
    pub returned_vector_candidate_count: usize,
    pub unique_graph_root_candidate_count: usize,
    pub selected_graph_root_count: usize,
    pub graph_root_omission_count: usize,
    pub graph_expansion: GraphExpansionTelemetry,
    #[serde(default)]
    pub selectivity: SelectivityTelemetry,
    pub section_pressure: Vec<SectionPressureSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectivityTelemetry {
    pub decision_count: usize,
    pub high_selectivity_count: usize,
    pub low_selectivity_supported_count: usize,
    pub low_selectivity_rejected_count: usize,
    pub fallback_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionTelemetry {
    pub attempted_root_count: usize,
    pub expanded_root_count: usize,
    pub missing_root_count: usize,
    pub expanded_object_count: usize,
    pub expanded_relation_count: usize,
    pub filtered_node_count: usize,
    pub bounded_failure_count: usize,
    pub bounded_failure_reasons: Vec<GraphExpansionBoundedFailureSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionBoundedFailureSummary {
    pub reason: GraphExpansionBoundedReason,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphExpansionBoundedReason {
    NodeLimit,
    Timeout,
    HubLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionPressureSummary {
    pub section: ContextPackSection,
    pub limit: usize,
    pub included_count: usize,
    pub omitted_by_limit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaleCandidateOmissionSummary {
    pub reason: StaleCandidateReason,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleOmissionSummary {
    pub reason: LifecycleFilterReason,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalTrace {
    pub vector_candidates: Vec<VectorCandidateTrace>,
    pub graph_relations: Vec<GraphRelationTrace>,
    #[serde(default)]
    pub graph_expansions: Vec<GraphExpansionTrace>,
    #[serde(default)]
    pub selectivity_decisions: Vec<SelectivityTrace>,
    pub lifecycle_filter_decisions: Vec<LifecycleFilterDecision>,
    pub stale_candidate_omissions: Vec<StaleCandidateOmission>,
    pub section_assignments: Vec<SectionAssignment>,
}

impl RetrievalTrace {
    pub fn empty() -> Self {
        Self {
            vector_candidates: Vec::new(),
            graph_relations: Vec::new(),
            graph_expansions: Vec::new(),
            selectivity_decisions: Vec::new(),
            lifecycle_filter_decisions: Vec::new(),
            stale_candidate_omissions: Vec::new(),
            section_assignments: Vec::new(),
        }
    }
}

impl Default for RetrievalTrace {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryObjectRef {
    pub object_type: ObjectType,
    pub id: MemoryId,
}

impl MemoryObjectRef {
    pub const fn new(object_type: ObjectType, id: MemoryId) -> Self {
        Self { object_type, id }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorCandidateTrace {
    pub object: MemoryObjectRef,
    pub score: f32,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRelationTrace {
    pub from: MemoryObjectRef,
    pub to: MemoryObjectRef,
    pub relation: RelationType,
    pub proximity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionTrace {
    pub root: MemoryObjectRef,
    pub object_count: usize,
    pub relation_count: usize,
    pub filtered_node_count: usize,
    pub bounded_failure: Option<GraphExpansionBoundedFailureTrace>,
    pub outcome: GraphExpansionOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectivityTrace {
    pub root: MemoryObjectRef,
    pub relation: RelationType,
    pub object_type: ObjectType,
    #[serde(default)]
    pub count_scope: SelectivityCountScope,
    pub score: Option<f64>,
    pub entity_count: Option<u64>,
    pub global_count: Option<u64>,
    pub support_factor: f64,
    pub chosen_fanout: usize,
    pub max_fanout: usize,
    pub decision: SelectivityDecision,
    pub fallback: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectivityCountScope {
    #[default]
    Current,
    Active,
    Total,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectivityDecision {
    HighSelectivity,
    LowSelectivitySupported,
    LowSelectivityRejected,
    ConservativeFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionBoundedFailureTrace {
    pub reason: GraphExpansionBoundedReason,
    pub at: Option<MemoryObjectRef>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphExpansionOutcome {
    Expanded,
    MissingRoot,
    Bounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleFilterDecision {
    pub object: MemoryObjectRef,
    pub retention_state: Option<RetentionState>,
    pub is_current: Option<bool>,
    pub superseded_by: Vec<MemoryId>,
    pub action: LifecycleFilterAction,
    pub reason: LifecycleFilterReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleFilterAction {
    Included,
    Omitted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleFilterReason {
    Active,
    ArchivedIncludedByPolicy,
    SuppressedIncludedByPolicy,
    DeletedIncludedByPolicy,
    NonCurrentIncludedByPolicy,
    SupersededIncludedByPolicy,
    ArchivedOmitted,
    SuppressedOmitted,
    DeletedOmitted,
    NonCurrentOmitted,
    SupersededOmitted,
    GraphObjectMissing,
    GraphExpansionBounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaleCandidateOmission {
    pub candidate: MemoryObjectRef,
    pub vector_score: Option<f32>,
    pub reason: StaleCandidateReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StaleCandidateReason {
    GraphObjectMissing,
    LifecycleMismatch,
    CurrentnessMismatch,
    Superseded,
    SectionLimit,
    GraphExpansionBounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionAssignment {
    pub object: MemoryObjectRef,
    pub section: ContextPackSection,
    pub rank: Option<usize>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextPackSection {
    ActiveThreads,
    RelevantEpisodes,
    SalientObservations,
    DerivedMemories,
    Preferences,
    RelationshipNotes,
    OpenLoops,
    Commitments,
    CharacterSignals,
    Omitted,
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use crate::api::types::domain::{DerivedType, Modality, Stability};

    fn memory_id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn episode(id: MemoryId) -> Episode {
        Episode {
            id,
            object_type: ObjectType::Episode,
            modality: Modality::Chat,
            source_conversation_id: Some("conversation-42".to_owned()),
            started_at: Some(timestamp("2026-04-29T10:00:00Z")),
            ended_at: Some(timestamp("2026-04-29T10:05:00Z")),
            participant_entity_ids: Vec::new(),
            summary: "Discussed context packs.".to_owned(),
            raw_ref: Some("raw://conversation/42#episode".to_owned()),
            salience_score: 0.8,
            retention_state: RetentionState::Active,
            created_at: timestamp("2026-04-29T10:06:00Z"),
            schema_version: "test_schema".to_owned(),
        }
    }

    fn observation(id: MemoryId, episode_id: MemoryId) -> Observation {
        Observation {
            id,
            object_type: ObjectType::Observation,
            episode_id,
            speaker_entity_id: None,
            observed_at: Some(timestamp("2026-04-29T10:01:00Z")),
            modality: Modality::Chat,
            text: "Concise observation excerpt.".to_owned(),
            raw_ref: Some("raw://conversation/42#turn-2".to_owned()),
            salience_score: 0.7,
            retention_state: RetentionState::Active,
            created_at: timestamp("2026-04-29T10:06:01Z"),
            schema_version: "test_schema".to_owned(),
        }
    }

    fn derived_memory(id: MemoryId, source_episode_id: MemoryId) -> DerivedMemory {
        DerivedMemory {
            id,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::UserPreference,
            text: "Prefers compact retrieval rationale.".to_owned(),
            derived_from_episode_ids: vec![source_episode_id],
            derived_from_observation_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655442010")],
            thread_ids: Vec::new(),
            entity_ids: Vec::new(),
            confidence: 0.9,
            salience_score: 0.7,
            stability: Stability::High,
            is_current: true,
            supersedes: Vec::new(),
            retention_state: RetentionState::Active,
            created_at: timestamp("2026-04-29T10:07:00Z"),
            updated_at: timestamp("2026-04-29T10:08:00Z"),
            schema_version: "test_schema".to_owned(),
        }
    }

    #[test]
    fn retrieval_context_serializes_with_defaults() {
        let context = RetrievalContext::new("What should I remember for this conversation?")
            .with_current_context("The user is planning a retrieval feature.")
            .with_trace();

        let encoded = serde_json::to_string(&context).unwrap();
        let decoded: RetrievalContext = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, context);
        assert_eq!(
            decoded.object_type_defaults,
            vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
                ObjectType::MemoryThread,
                ObjectType::Entity,
            ]
        );
        assert!(decoded.include_trace);
    }

    #[test]
    fn default_policy_excludes_non_active_and_stale_derived_memories() {
        let policy = RetrievalLifecyclePolicy::default();
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442000");
        let mut memory = derived_memory(
            memory_id("550e8400-e29b-41d4-a716-446655442001"),
            episode_id,
        );

        assert!(policy.allows_retention_state(RetentionState::Active));
        assert!(!policy.allows_retention_state(RetentionState::Archived));
        assert!(!policy.allows_retention_state(RetentionState::Suppressed));
        assert!(!policy.allows_retention_state(RetentionState::Deleted));
        assert!(policy.allows_derived_memory(&memory));

        memory.is_current = false;
        assert!(!policy.allows_derived_memory(&memory));

        let policy = RetrievalLifecyclePolicy {
            include_non_current: true,
            ..RetrievalLifecyclePolicy::default()
        };
        assert!(policy.allows_derived_memory(&memory));
    }

    #[test]
    fn default_policy_allows_current_derived_memory_that_supersedes_older_memory() {
        let policy = RetrievalLifecyclePolicy::default();
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442050");
        let mut memory = derived_memory(
            memory_id("550e8400-e29b-41d4-a716-446655442051"),
            episode_id,
        );
        memory.supersedes = vec![memory_id("550e8400-e29b-41d4-a716-446655442052")];

        assert!(policy.allows_derived_memory(&memory));
    }

    #[test]
    fn derived_memory_currentness_policy_is_independent_of_local_supersedes_list() {
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442060");
        let mut memory = derived_memory(
            memory_id("550e8400-e29b-41d4-a716-446655442061"),
            episode_id,
        );
        memory.is_current = false;
        memory.supersedes = vec![memory_id("550e8400-e29b-41d4-a716-446655442062")];

        assert!(!RetrievalLifecyclePolicy::default().allows_derived_memory(&memory));

        let policy = RetrievalLifecyclePolicy {
            include_non_current: true,
            ..RetrievalLifecyclePolicy::default()
        };
        assert!(policy.allows_derived_memory(&memory));
    }

    #[test]
    fn include_superseded_is_for_graph_superseded_by_evidence_not_local_supersedes() {
        let object = MemoryObjectRef::new(
            ObjectType::DerivedMemory,
            memory_id("550e8400-e29b-41d4-a716-446655442070"),
        );
        let newer_memory_id = memory_id("550e8400-e29b-41d4-a716-446655442071");
        let decision = LifecycleFilterDecision {
            object,
            retention_state: Some(RetentionState::Active),
            is_current: Some(false),
            superseded_by: vec![newer_memory_id],
            action: LifecycleFilterAction::Included,
            reason: LifecycleFilterReason::SupersededIncludedByPolicy,
        };

        assert_eq!(decision.superseded_by, vec![newer_memory_id]);
    }

    #[test]
    fn section_assignment_shape_reports_final_section() {
        let assignment = SectionAssignment {
            object: MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                memory_id("550e8400-e29b-41d4-a716-446655442020"),
            ),
            section: ContextPackSection::Preferences,
            rank: Some(2),
            reason: Some("derived type maps to preference section".to_owned()),
        };

        let encoded = serde_json::to_value(&assignment).unwrap();

        assert_eq!(encoded["section"], "preferences");
        assert_eq!(encoded["rank"], 2);
    }

    #[test]
    fn context_pack_preserves_source_references_without_raw_transcript_storage() {
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442030");
        let derived = derived_memory(
            memory_id("550e8400-e29b-41d4-a716-446655442031"),
            episode_id,
        );
        let included = IncludedDerivedMemory::from(derived.clone());
        let mut pack = ContinuityContextPack::empty();
        pack.relevant_episodes.push(episode(episode_id));
        pack.salient_observations.push(observation(
            memory_id("550e8400-e29b-41d4-a716-446655442010"),
            episode_id,
        ));
        pack.preferences.push(included);

        let encoded_value = serde_json::to_value(&pack).unwrap();
        let encoded = serde_json::to_string(&pack).unwrap();
        let decoded: ContinuityContextPack = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            decoded.relevant_episodes[0].raw_ref.as_deref(),
            Some("raw://conversation/42#episode")
        );
        assert_eq!(
            decoded.salient_observations[0].raw_ref.as_deref(),
            Some("raw://conversation/42#turn-2")
        );
        assert_eq!(decoded.preferences[0].source_episode_ids, vec![episode_id]);
        assert_eq!(
            decoded.preferences[0].source_observation_ids,
            derived.derived_from_observation_ids
        );
        assert_eq!(decoded.preferences[0].memory.text, derived.text);
        for raw_content_key in [
            "raw_transcript",
            "raw_text",
            "transcript",
            "source_transcript",
        ] {
            assert!(!json_contains_key(&encoded_value, raw_content_key));
        }
    }

    fn json_contains_key(value: &serde_json::Value, key: &str) -> bool {
        match value {
            serde_json::Value::Object(object) => {
                object.contains_key(key)
                    || object.values().any(|value| json_contains_key(value, key))
            }
            serde_json::Value::Array(values) => {
                values.iter().any(|value| json_contains_key(value, key))
            }
            _ => false,
        }
    }

    #[test]
    fn trace_can_report_candidate_graph_lifecycle_stale_and_section_details() {
        let candidate_id = memory_id("550e8400-e29b-41d4-a716-446655442040");
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442041");
        let candidate = MemoryObjectRef::new(ObjectType::DerivedMemory, candidate_id);
        let episode = MemoryObjectRef::new(ObjectType::Episode, episode_id);
        let trace = RetrievalTrace {
            vector_candidates: vec![VectorCandidateTrace {
                object: candidate,
                score: 0.82,
                rank: 1,
            }],
            graph_relations: vec![GraphRelationTrace {
                from: candidate,
                to: episode,
                relation: RelationType::DerivedFrom,
                proximity: 1,
            }],
            graph_expansions: vec![GraphExpansionTrace {
                root: candidate,
                object_count: 2,
                relation_count: 1,
                filtered_node_count: 0,
                bounded_failure: Some(GraphExpansionBoundedFailureTrace {
                    reason: GraphExpansionBoundedReason::NodeLimit,
                    at: Some(episode),
                }),
                outcome: GraphExpansionOutcome::Bounded,
            }],
            selectivity_decisions: Vec::new(),
            lifecycle_filter_decisions: vec![LifecycleFilterDecision {
                object: candidate,
                retention_state: Some(RetentionState::Active),
                is_current: Some(true),
                superseded_by: Vec::new(),
                action: LifecycleFilterAction::Included,
                reason: LifecycleFilterReason::Active,
            }],
            stale_candidate_omissions: vec![StaleCandidateOmission {
                candidate: MemoryObjectRef::new(ObjectType::Observation, episode_id),
                vector_score: Some(0.44),
                reason: StaleCandidateReason::GraphObjectMissing,
            }],
            section_assignments: vec![SectionAssignment {
                object: candidate,
                section: ContextPackSection::Preferences,
                rank: Some(1),
                reason: None,
            }],
        };

        let encoded = serde_json::to_string(&trace).unwrap();
        let decoded: RetrievalTrace = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.vector_candidates[0].score, 0.82);
        assert_eq!(
            decoded.graph_relations[0].relation,
            RelationType::DerivedFrom
        );
        assert_eq!(
            decoded.graph_expansions[0]
                .bounded_failure
                .as_ref()
                .unwrap()
                .reason,
            GraphExpansionBoundedReason::NodeLimit
        );
        assert_eq!(
            decoded.lifecycle_filter_decisions[0].reason,
            LifecycleFilterReason::Active
        );
        assert_eq!(
            decoded.stale_candidate_omissions[0].reason,
            StaleCandidateReason::GraphObjectMissing
        );
        assert_eq!(
            decoded.section_assignments[0].section,
            ContextPackSection::Preferences
        );
    }

    #[test]
    fn retrieval_trace_deserializes_old_payload_without_graph_expansions() {
        let encoded = r#"{
            "vector_candidates": [],
            "graph_relations": [],
            "lifecycle_filter_decisions": [],
            "stale_candidate_omissions": [],
            "section_assignments": []
        }"#;

        let decoded: RetrievalTrace = serde_json::from_str(encoded).unwrap();

        assert!(decoded.graph_expansions.is_empty());
        assert!(decoded.selectivity_decisions.is_empty());
    }

    #[test]
    fn retrieval_telemetry_serializes_with_backend_agnostic_bounds() {
        let telemetry = RetrievalTelemetry {
            query_embedding_dimension: 3,
            returned_vector_candidate_count: 4,
            unique_graph_root_candidate_count: 3,
            selected_graph_root_count: 2,
            graph_root_omission_count: 1,
            graph_expansion: GraphExpansionTelemetry {
                attempted_root_count: 2,
                bounded_failure_count: 1,
                bounded_failure_reasons: vec![GraphExpansionBoundedFailureSummary {
                    reason: GraphExpansionBoundedReason::HubLimit,
                    count: 1,
                }],
                ..GraphExpansionTelemetry::default()
            },
            selectivity: SelectivityTelemetry {
                decision_count: 2,
                high_selectivity_count: 1,
                low_selectivity_supported_count: 1,
                low_selectivity_rejected_count: 0,
                fallback_count: 0,
            },
            section_pressure: vec![SectionPressureSummary {
                section: ContextPackSection::SalientObservations,
                limit: 16,
                included_count: 16,
                omitted_by_limit_count: 2,
            }],
            ..RetrievalTelemetry::default()
        };
        let mut rationale = RetrievalRationale::new("telemetry example");
        rationale.telemetry = telemetry.clone();

        let encoded = serde_json::to_value(&rationale).unwrap();
        let decoded: RetrievalRationale = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(decoded.telemetry, telemetry);
        assert_eq!(
            encoded["telemetry"]["graph_expansion"]["bounded_failure_reasons"][0]["reason"],
            "hub_limit"
        );
        assert_eq!(encoded["telemetry"]["selectivity"]["decision_count"], 2);
    }

    #[test]
    fn retrieval_telemetry_default_preserves_retrieval_defaults() {
        let telemetry = RetrievalTelemetry::default();

        assert_eq!(
            telemetry.configured_candidate_limits,
            RetrievalCandidateLimits::default()
        );
        assert_eq!(
            telemetry.configured_graph_limits,
            RetrievalGraphLimits::default()
        );
        assert_eq!(
            telemetry.configured_section_limits,
            ContinuitySectionLimits::default()
        );
    }
}
