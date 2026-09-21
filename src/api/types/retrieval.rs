use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    DerivedMemory, Episode, GraphExpansionBoundedFailureTrace, GraphExpansionBoundedReason,
    GraphFailureMode, MemoryId, MemoryObjectRef, MemoryThread, ObjectType, Observation,
    RelationType, RetentionState, Scene, ThreadStatus, VectorSurface,
};
use crate::errors::{ConfigValidationError, ConfigValidationReason};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalContext {
    #[serde(default = "Scene::now")]
    pub scene: Scene,
    pub topic: Option<String>,
    pub activity: Option<ActivityRef>,
    pub candidate_limits: RetrievalCandidateLimits,
    pub graph_limits: RetrievalGraphLimits,
    pub section_limits: ContinuitySectionLimits,
    /// Per-kind room at candidate, root and section caps. A calibration knob for
    /// measured defaults; applications are not expected to set it. Five people
    /// share one participant floor, rather than receiving one floor each.
    pub cue_floors: RetrievalCueFloors,
    pub lifecycle_policy: RetrievalLifecyclePolicy,
    pub include_trace: bool,
    /// Object types admitted by vector candidate recall. Graph traversal has its own limits.
    pub object_type_defaults: Vec<ObjectType>,
}

impl RetrievalContext {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: Some(topic.into()),
            ..Self::default()
        }
    }

    pub fn with_scene(mut self, scene: Scene) -> Self {
        self.scene = scene;
        self
    }

    pub fn with_trace(mut self) -> Self {
        self.include_trace = true;
        self
    }

    pub fn with_activity(mut self, activity: ActivityRef) -> Self {
        self.activity = Some(activity);
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.object_type_defaults.is_empty() {
            return Err(ConfigValidationError {
                keys: vec!["object_type_defaults"],
                reason: ConfigValidationReason::OutOfDomain {
                    expected: "at least one retrieval object type",
                    actual: "[]".to_owned(),
                },
            });
        }

        Ok(())
    }
}

impl Default for RetrievalContext {
    fn default() -> Self {
        Self {
            scene: Scene::now(),
            topic: None,
            activity: None,
            candidate_limits: RetrievalCandidateLimits::default(),
            graph_limits: RetrievalGraphLimits::default(),
            section_limits: ContinuitySectionLimits::default(),
            cue_floors: RetrievalCueFloors::default(),
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

/// Calibration values for measured defaults, provisionally one slot per cue kind.
/// Applications are not expected to set these. Floors apply per kind, not per
/// person or place: five people share the participant floor.
///
/// After reservations, spare room follows score order except at root selection,
/// where present kinds share turns. A zero floor removes only the reservation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalCueFloors {
    pub participant: usize,
    pub place: usize,
    pub activity: usize,
    pub topic: usize,
}

impl Default for RetrievalCueFloors {
    fn default() -> Self {
        Self {
            participant: 1,
            place: 1,
            activity: 1,
            topic: 1,
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
    pub failure_mode: GraphFailureMode,
    pub allowed_relation_types: Vec<RelationType>,
    /// Types allowed during graph traversal; independent of vector candidate scope.
    /// An empty list imposes no object-type restriction.
    pub allowed_object_types: Vec<ObjectType>,
}

impl Default for RetrievalGraphLimits {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_nodes: 96,
            max_fanout_per_node: 16,
            max_hub_edges: 64,
            timeout_ms: Some(250),
            failure_mode: GraphFailureMode::AllowPartialResults,
            allowed_relation_types: Vec::new(),
            allowed_object_types: vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
                ObjectType::MemoryThread,
                ObjectType::Entity,
            ],
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
    pub include_suppressed: bool,
    /// Applies to graph-verified supersession evidence reported as `superseded_by`.
    /// A derived memory's local `supersedes` list points to older memories it replaces.
    pub include_superseded: bool,
}

impl RetrievalLifecyclePolicy {
    pub fn allows_retention_state(self, retention_state: RetentionState) -> bool {
        match retention_state {
            RetentionState::Active => true,
            RetentionState::Suppressed => self.include_suppressed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrieveOutcome {
    /// An unset or empty part means not given. A scene is never complete: people
    /// can be present and unperceived.
    pub scene: Scene,
    pub activity: Option<ActivityResult>,
    pub scene_references: Vec<SceneReferenceResult>,
    pub memory_scenes: Vec<MemoryScenes>,
    pub pack: ContinuityContextPack,
    pub rationale: RetrievalRationale,
    pub trace: Option<RetrievalTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryScenes {
    pub memory: MemoryObjectRef,
    /// Empty means no recorded experience; unavailable sources are explicit entries.
    pub sources: Vec<SourceScene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceScene {
    Recorded {
        episode_id: MemoryId,
        scene: Scene,
    },
    Unavailable {
        source: MemoryObjectRef,
        reason: SourceSceneUnavailableReason,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceSceneUnavailableReason {
    Missing,
    Forgotten,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneReference {
    ParticipantKey { index: usize },
    ParticipantName { index: usize },
    ParticipantDescription { index: usize },
    SettingWords,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneReferenceResult {
    pub reference: SceneReference,
    pub resolution: SceneReferenceResolution,
    /// One entry per resolved notion, independent of retrieval caps. `None` means
    /// never met at or before the scene time; content cues and unknowns are empty.
    pub last_interactions: BTreeMap<MemoryId, Option<LastInteraction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastInteraction {
    pub episode_id: MemoryId,
    pub scene_time: DateTime<Utc>,
    /// Whole seconds from the recorded experience to the retrieval scene time.
    pub seconds_since: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneReferenceResolution {
    Resolved {
        notion_id: MemoryId,
    },
    Ambiguous {
        notion_ids: Vec<MemoryId>,
    },
    /// For a name, no notion is currently known by exactly this name, nothing more.
    /// For a key, no notion currently exists at that key.
    Unknown,
    ContentCue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ActivityRef {
    Thread(MemoryId),
    OpenLoop(MemoryId),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityResult {
    pub activity: ActivityRef,
    pub resolution: ActivityResolution,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityResolution {
    Found,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphRootSource {
    Vector,
    Participant,
    Activity,
    Place,
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
    /// Memories linked by Resolves or FulfillsCommitment; empty means unresolved.
    pub resolved_by: Vec<MemoryId>,
}

impl From<DerivedMemory> for IncludedDerivedMemory {
    fn from(memory: DerivedMemory) -> Self {
        Self {
            source_episode_ids: memory.derived_from_episode_ids.clone(),
            source_observation_ids: memory.derived_from_observation_ids.clone(),
            resolved_by: Vec::new(),
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
    pub configured_object_types: Vec<ObjectType>,
    pub configured_lifecycle_policy: RetrievalLifecyclePolicy,
    /// The store-compatible dimension of content queries, or zero if none was embedded.
    pub query_embedding_dimension: usize,
    /// Unique objects after merging content searches at their best score and applying the candidate limit.
    pub returned_vector_candidate_count: usize,
    /// Weakest contributing search verdict, with that search's own counters.
    pub vector_recall_completeness: VectorRecallCompleteness,
    /// Distinct participant roots plus merged content roots, before the root limit.
    pub unique_graph_root_candidate_count: usize,
    pub selected_graph_root_count: usize,
    pub graph_root_omission_count: usize,
    pub graph_expansion: GraphExpansionTelemetry,
    pub selectivity: SelectivityTelemetry,
    pub section_pressure: Vec<SectionPressureSummary>,
}

/// Completeness of vector search. A retrieval with several distinct content queries
/// reports the weakest contributing verdict with that search's own counters.
/// Equally weak verdicts retain the first search in cue order.
///
/// `NotRequested` means no vector search ran. `Exhaustive` means every record in
/// the requested scope was scored through a path the adapter knows to be exhaustive,
/// the cutoff cohort closed, and `scanned` is the number of scoped records that path
/// actually scored. This includes an unindexed scan or a full-scope scroll such as
/// the zero-norm path.
/// `BoundaryTieClosed` means the shared fetch loop closed the cutoff score cohort
/// within an index-produced result prefix; `fetched` is the size of the final prefix
/// read, not the cumulative rows read across the loop's growth steps.
/// `BoundaryTieOpen` means the cohort was still open at `fetch_bound`; `fetched`
/// is likewise the size of the final prefix read. Indexed boundary verdicts are
/// deterministic only for the index-returned prefix and never claim population-level
/// completeness.
///
/// ```
/// use character_memory::api::types::VectorRecallCompleteness as ApiCompleteness;
/// use character_memory::VectorRecallCompleteness;
///
/// let _: VectorRecallCompleteness = ApiCompleteness::NotRequested;
/// ```
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VectorRecallCompleteness {
    #[default]
    NotRequested,
    Exhaustive {
        scanned: usize,
    },
    BoundaryTieClosed {
        fetched: usize,
    },
    BoundaryTieOpen {
        fetched: usize,
        fetch_bound: usize,
    },
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
#[non_exhaustive]
pub struct RetrievalTrace {
    pub vector_candidates: Vec<VectorCandidateTrace>,
    /// Recallable scene matches left out by each description search's occasion limit.
    /// Participant words share one search and therefore one count.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scene_cue_omitted_counts: BTreeMap<CueKind, usize>,
    pub floor_admissions: Vec<CueFloorAdmission>,
    pub graph_relations: Vec<GraphRelationTrace>,
    pub graph_expansions: Vec<GraphExpansionTrace>,
    pub fanout_utilization: Vec<FanoutUtilizationTrace>,
    pub selectivity_decisions: Vec<SelectivityTrace>,
    pub lifecycle_filter_decisions: Vec<LifecycleFilterDecision>,
    pub stale_candidate_omissions: Vec<StaleCandidateOmission>,
    pub section_assignments: Vec<SectionAssignment>,
}

impl RetrievalTrace {
    pub fn empty() -> Self {
        Self {
            vector_candidates: Vec::new(),
            scene_cue_omitted_counts: BTreeMap::new(),
            floor_admissions: Vec::new(),
            graph_relations: Vec::new(),
            graph_expansions: Vec::new(),
            fanout_utilization: Vec::new(),
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

/// A reserved or spare cue turn admitted this object outside the stage's original capped prefix.
/// Earlier-stage admissions remain here even if the object is omitted later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CueFloorAdmission {
    pub object: MemoryObjectRef,
    pub stage: CueFloorStage,
    pub cue_kind: CueKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CueFloorStage {
    CandidateMerge,
    GraphRoots,
    Section { section: ContextPackSection },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorCandidateTrace {
    pub object: MemoryObjectRef,
    pub surface: VectorSurface,
    pub score: f32,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRelationTrace {
    pub link_id: MemoryId,
    pub from: MemoryObjectRef,
    pub to: MemoryObjectRef,
    pub relation: RelationType,
    pub proximity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionTrace {
    pub source: GraphRootSource,
    pub root: MemoryObjectRef,
    pub object_count: usize,
    pub relation_count: usize,
    pub filtered_node_count: usize,
    pub bounded_failure: Option<GraphExpansionBoundedFailureTrace>,
    pub outcome: GraphExpansionOutcome,
}

/// Reports pre-limit counts only for nodes the bounded expansion actually expanded.
///
/// Trace roots are a subset of returned expansion objects: nodes admitted at max depth are returned but are not expanded and therefore emit no utilization rows.
/// Omission counts include links dropped by the global hub cap and relation/object fanout caps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutUtilizationTrace {
    pub root: MemoryObjectRef,
    pub relation: RelationType,
    pub object_type: ObjectType,
    pub configured_cap: usize,
    pub selected_cap: usize,
    pub retained_count: usize,
    pub omitted_by_fanout_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectivityTrace {
    pub root: MemoryObjectRef,
    pub relation: RelationType,
    pub object_type: ObjectType,
    pub count_scope: SelectivityCountScope,
    pub score: Option<f64>,
    /// Distinct episodes involving the notion for Involves/Episode and Mentions/Observation;
    /// otherwise the notion's relation/object edge count.
    pub entity_count: Option<u64>,
    /// All episodes in the count scope for the two participant paths;
    /// otherwise the global relation/object edge count.
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
    /// About fanout is not reduced when the scene explicitly names the subject.
    SkippedSceneNamedRoot,
    HighSelectivity,
    LowSelectivitySupported,
    LowSelectivityRejected,
    ConservativeFallback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphExpansionOutcome {
    RootLimit,
    Expanded,
    MissingRoot,
    Bounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleFilterDecision {
    pub object: MemoryObjectRef,
    pub retention_state: Option<RetentionState>,
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
    SuppressedIncludedByPolicy,
    SupersededIncludedByPolicy,
    SuppressedOmitted,
    SupersededOmitted,
    GraphObjectMissing,
    GraphExpansionBounded,
    ResolvedOmitted,
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
    Superseded,
    SectionLimit,
    GraphExpansionBounded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SectionAssignment {
    pub object: MemoryObjectRef,
    pub section: ContextPackSection,
    pub rank: Option<usize>,
    pub reason: SectionAssignmentReason,
    pub cue_kinds: BTreeSet<CueKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SectionAssignmentReason {
    Selected {
        scores: SectionScoreComponents,
    },
    OmittedByLimit {
        intended_section: ContextPackSection,
        scores: SectionScoreComponents,
    },
    OmittedNonActiveThread {
        thread_status: ThreadStatus,
    },
    OmittedNoPromptSection {
        object_type: ObjectType,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SectionScoreComponents {
    pub final_score: f32,
    pub cue_score: Option<f32>,
    pub graph_score: Option<f32>,
    pub salience_score: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CueKind {
    Topic,
    Participant,
    Place,
    Activity,
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

    use crate::domain::{DerivedType, Modality};

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
            scene: crate::domain::Scene {
                setting: crate::domain::SceneSetting {
                    key: Some("conversation-42".to_owned()),
                    words: None,
                },

                ..crate::domain::Scene::at(timestamp("2026-04-29T10:00:00Z"))
            },
            ended_at: Some(timestamp("2026-04-29T10:05:00Z")),
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
            scope_keys: Vec::new(),
            assertions: Vec::new(),
            given_by_application: false,
            id,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::UserPreference,
            text: "Prefers compact retrieval rationale.".to_owned(),
            derived_from_episode_ids: vec![source_episode_id],
            derived_from_observation_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655442010")],
            thread_ids: Vec::new(),
            entity_ids: Vec::new(),
            salience_score: 0.7,
            supersedes: Vec::new(),
            retention_state: RetentionState::Active,
            created_at: timestamp("2026-04-29T10:07:00Z"),
            updated_at: timestamp("2026-04-29T10:08:00Z"),
            schema_version: "test_schema".to_owned(),
        }
    }

    #[test]
    fn retrieval_context_defaults_to_every_object_type() {
        let context = RetrievalContext::new("What should I remember for this conversation?");

        assert_eq!(
            context.object_type_defaults,
            vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
                ObjectType::MemoryThread,
            ]
        );
    }

    #[test]
    fn suppressed_retention_requires_explicit_inclusion() {
        let policy = RetrievalLifecyclePolicy::default();
        assert!(policy.allows_retention_state(RetentionState::Active));
        assert!(!policy.allows_retention_state(RetentionState::Suppressed));
        assert!(RetrievalLifecyclePolicy {
            include_suppressed: true,
            ..policy
        }
        .allows_retention_state(RetentionState::Suppressed));
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
            reason: SectionAssignmentReason::Selected {
                scores: SectionScoreComponents {
                    final_score: 0.75,
                    cue_score: Some(0.8),
                    graph_score: Some(0.5),
                    salience_score: None,
                },
            },
            cue_kinds: BTreeSet::from([CueKind::Place, CueKind::Topic, CueKind::Participant]),
        };

        let encoded = serde_json::to_value(&assignment).unwrap();

        assert_eq!(encoded["section"], "preferences");
        assert_eq!(encoded["rank"], 2);
        assert_eq!(encoded["reason"]["kind"], "selected");
        assert_eq!(encoded["reason"]["scores"]["final_score"], 0.75);
        assert_eq!(
            encoded["cue_kinds"],
            serde_json::json!(["topic", "participant", "place"])
        );
    }

    #[test]
    fn context_pack_preserves_source_references_without_raw_transcript_storage() {
        let episode_id = memory_id("550e8400-e29b-41d4-a716-446655442030");
        let derived = derived_memory(
            memory_id("550e8400-e29b-41d4-a716-446655442031"),
            episode_id,
        );
        let included = IncludedDerivedMemory::from(derived);
        let mut pack = ContinuityContextPack::empty();
        pack.relevant_episodes.push(episode(episode_id));
        pack.salient_observations.push(observation(
            memory_id("550e8400-e29b-41d4-a716-446655442010"),
            episode_id,
        ));
        pack.preferences.push(included);

        let encoded_value = serde_json::to_value(&pack).unwrap();

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
}
