// Vector candidate query surface. Some filters are exercised by live
// adapters while deterministic tests use narrower subsets.
use std::collections::{hash_map::Entry, HashMap};

use chrono::{DateTime, Utc};

use crate::api::types::default_retrieval_object_types;
use crate::domain::{MemoryId, ObjectType, RetentionState};

use super::{VectorPayloadHints, VectorRelationshipHints};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VectorSurface {
    Summary,
    Text,
    Name,
    DerivedText,
    Query,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmbeddingInput {
    pub(crate) object_id: Option<MemoryId>,
    pub(crate) object_type: Option<ObjectType>,
    pub(crate) surface: VectorSurface,
    pub(crate) text: String,
}

impl EmbeddingInput {
    pub(crate) fn new(
        object_id: Option<MemoryId>,
        object_type: Option<ObjectType>,
        surface: VectorSurface,
        text: impl Into<String>,
    ) -> Self {
        Self {
            object_id,
            object_type,
            surface,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
// Test fakes and diagnostics use candidate records selectively; remove when all vector stores expose diagnostics.
#[allow(dead_code)]
pub(crate) struct VectorCandidateRecord {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) surface: VectorSurface,
    pub(crate) embedding: Vec<f32>,
    pub(crate) retention_state: Option<RetentionState>,
    pub(crate) is_current: Option<bool>,
    pub(crate) relationship_hints: VectorRelationshipHints,
    pub(crate) payload_hints: VectorPayloadHints,
}

impl VectorCandidateRecord {
    // Deterministic fakes build candidate records directly; remove when fakes use VectorRecord only.
    #[allow(dead_code)]
    pub(crate) fn new(
        object_id: MemoryId,
        object_type: ObjectType,
        surface: VectorSurface,
        embedding: Vec<f32>,
    ) -> Self {
        Self {
            object_id,
            object_type,
            surface,
            embedding,
            retention_state: None,
            is_current: None,
            relationship_hints: VectorRelationshipHints::default(),
            payload_hints: VectorPayloadHints::default(),
        }
    }

    // Payload-filter fixtures use explicit hints; remove when fixture construction moves to VectorRecord.
    #[allow(dead_code)]
    pub(crate) fn with_filter_hints(
        mut self,
        retention_state: Option<RetentionState>,
        is_current: Option<bool>,
        relationship_hints: VectorRelationshipHints,
        payload_hints: VectorPayloadHints,
    ) -> Self {
        self.retention_state = retention_state;
        self.is_current = is_current;
        self.relationship_hints = relationship_hints;
        self.payload_hints = payload_hints;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateSearch {
    pub(crate) query_embedding: Vec<f32>,
    pub(crate) limit: usize,
    pub(crate) object_types: Vec<ObjectType>,
    pub(crate) filters: VectorCandidateFilters,
}

impl VectorCandidateSearch {
    pub(crate) fn new(query_embedding: Vec<f32>, limit: usize) -> Self {
        Self {
            query_embedding,
            limit,
            object_types: Vec::new(),
            filters: VectorCandidateFilters::default(),
        }
    }

    pub(crate) fn with_object_types(mut self, object_types: Vec<ObjectType>) -> Self {
        self.object_types = object_types;
        self
    }

    // Default recall scope is a convenience for tests and future callers; remove if all callers pass explicit types.
    #[allow(dead_code)]
    pub(crate) fn with_default_object_types(mut self) -> Self {
        self.object_types = default_vector_candidate_object_types();
        self
    }

    pub(crate) fn with_filters(mut self, filters: VectorCandidateFilters) -> Self {
        self.filters = filters;
        self
    }
}

// Default recall scope is retained for query-builder callers; remove if query builders stop exposing defaults.
#[allow(dead_code)]
pub(crate) fn default_vector_candidate_object_types() -> Vec<ObjectType> {
    default_retrieval_object_types()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct VectorCandidateFilters {
    pub(crate) retention_states: Vec<RetentionState>,
    pub(crate) is_current: Option<bool>,
    pub(crate) is_superseded: Option<bool>,
    pub(crate) thread_ids: Vec<MemoryId>,
    pub(crate) entity_ids: Vec<MemoryId>,
    pub(crate) episode_ids: Vec<MemoryId>,
    pub(crate) time_ranges: Vec<VectorTimeRangeFilter>,
}

impl VectorCandidateFilters {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // Lifecycle prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn with_retention_states(mut self, retention_states: Vec<RetentionState>) -> Self {
        self.retention_states = retention_states;
        self
    }

    // Lifecycle prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn current_only(mut self) -> Self {
        self.is_current = Some(true);
        self.is_superseded = Some(false);
        self
    }

    pub(crate) fn has_currentness_filters(&self) -> bool {
        self.is_current.is_some() || self.is_superseded.is_some()
    }

    // Relationship prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn with_thread_ids(mut self, thread_ids: Vec<MemoryId>) -> Self {
        self.thread_ids = thread_ids;
        self
    }

    // Relationship prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn with_entity_ids(mut self, entity_ids: Vec<MemoryId>) -> Self {
        self.entity_ids = entity_ids;
        self
    }

    // Relationship prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn with_episode_ids(mut self, episode_ids: Vec<MemoryId>) -> Self {
        self.episode_ids = episode_ids;
        self
    }

    // Time prefilter builders are used by adapter and fixture subsets; remove when all callers build filters directly.
    #[allow(dead_code)]
    pub(crate) fn with_time_range(mut self, time_range: VectorTimeRangeFilter) -> Self {
        self.time_ranges.push(time_range);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VectorTimeRangeFilter {
    pub(crate) field: VectorTimeField,
    pub(crate) after: Option<DateTime<Utc>>,
    pub(crate) before: Option<DateTime<Utc>>,
}

impl VectorTimeRangeFilter {
    // Time prefilter builders are used by adapter and fixture subsets; remove when all callers build ranges directly.
    #[allow(dead_code)]
    pub(crate) fn new(
        field: VectorTimeField,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            field,
            after,
            before,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// The query model reserves all payload time fields; remove variants only with matching payload/index changes.
#[allow(dead_code)]
pub(crate) enum VectorTimeField {
    Created,
    Updated,
    Started,
    Ended,
    Observed,
    LastTouched,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateMatch {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) surface: VectorSurface,
    pub(crate) score: f32,
}

impl VectorCandidateMatch {
    pub(crate) fn new(
        object_id: MemoryId,
        object_type: ObjectType,
        surface: VectorSurface,
        score: f32,
    ) -> Self {
        Self {
            object_id,
            object_type,
            surface,
            score,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalCandidates(Vec<VectorCandidateMatch>);

impl CanonicalCandidates {
    pub(crate) fn new(candidates: impl IntoIterator<Item = VectorCandidateMatch>) -> Self {
        Self(canonicalize_vector_candidates(candidates))
    }

    pub(crate) fn truncated(mut self, limit: usize) -> Self {
        self.0.truncate(limit);
        self
    }
}

impl std::ops::Deref for CanonicalCandidates {
    type Target = [VectorCandidateMatch];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn canonicalize_vector_candidates(
    candidates: impl IntoIterator<Item = VectorCandidateMatch>,
) -> Vec<VectorCandidateMatch> {
    let mut by_identity = HashMap::new();
    for candidate in candidates {
        let identity = (
            candidate.object_id,
            object_type_rank(candidate.object_type),
            vector_surface_rank(candidate.surface),
        );
        match by_identity.entry(identity) {
            Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
            Entry::Occupied(mut entry) if candidate.score.total_cmp(&entry.get().score).is_gt() => {
                entry.insert(candidate);
            }
            Entry::Occupied(_) => {}
        }
    }

    let mut candidates = by_identity.into_values().collect::<Vec<_>>();
    candidates.sort_by(compare_vector_candidates);
    candidates
}

fn compare_vector_candidates(
    left: &VectorCandidateMatch,
    right: &VectorCandidateMatch,
) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| object_type_rank(left.object_type).cmp(&object_type_rank(right.object_type)))
        .then_with(|| left.object_id.cmp(&right.object_id))
        .then_with(|| vector_surface_rank(left.surface).cmp(&vector_surface_rank(right.surface)))
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

fn vector_surface_rank(surface: VectorSurface) -> u8 {
    match surface {
        VectorSurface::Summary => 0,
        VectorSurface::Text => 1,
        VectorSurface::Name => 2,
        VectorSurface::DerivedText => 3,
        VectorSurface::Query => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_candidate_record_keeps_domain_identity_and_embedding_surface() {
        let object_id = MemoryId::new_v4();
        let record = VectorCandidateRecord::new(
            object_id,
            ObjectType::Observation,
            VectorSurface::Text,
            vec![0.1, 0.2, 0.3],
        );

        assert_eq!(record.object_id, object_id);
        assert_eq!(record.object_type, ObjectType::Observation);
        assert_eq!(record.surface, VectorSurface::Text);
        assert_eq!(record.embedding, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn vector_candidate_search_can_scope_by_canonical_object_types() {
        let search = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_object_types(vec![ObjectType::Episode, ObjectType::DerivedMemory]);

        assert_eq!(search.query_embedding, vec![1.0, 0.0]);
        assert_eq!(search.limit, 10);
        assert_eq!(
            search.object_types,
            vec![ObjectType::Episode, ObjectType::DerivedMemory]
        );
    }

    #[test]
    fn canonical_candidates_dedupe_identity_at_highest_score_and_totally_order_ties() {
        let episode_id = MemoryId::from_u128(2);
        let observation_id = MemoryId::from_u128(1);
        let candidates = vec![
            VectorCandidateMatch::new(episode_id, ObjectType::Episode, VectorSurface::Summary, 0.4),
            VectorCandidateMatch::new(
                observation_id,
                ObjectType::Observation,
                VectorSurface::Text,
                0.9,
            ),
            VectorCandidateMatch::new(episode_id, ObjectType::Episode, VectorSurface::Summary, 0.9),
        ];

        let canonical = CanonicalCandidates::new(candidates);

        assert_eq!(canonical.len(), 2);
        assert_eq!(canonical[0].object_id, episode_id);
        assert_eq!(canonical[0].score, 0.9);
        assert_eq!(canonical[1].object_id, observation_id);
    }

    #[test]
    fn vector_candidate_search_can_express_default_types_and_payload_hint_filters() {
        let thread_id = MemoryId::new_v4();
        let entity_id = MemoryId::new_v4();
        let episode_id = MemoryId::new_v4();
        let timestamp = DateTime::parse_from_rfc3339("2026-04-29T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let filters = VectorCandidateFilters::new()
            .with_retention_states(vec![RetentionState::Active])
            .current_only()
            .with_thread_ids(vec![thread_id])
            .with_entity_ids(vec![entity_id])
            .with_episode_ids(vec![episode_id])
            .with_time_range(VectorTimeRangeFilter::new(
                VectorTimeField::Observed,
                Some(timestamp),
                None,
            ));
        let search = VectorCandidateSearch::new(vec![1.0, 0.0], 12)
            .with_default_object_types()
            .with_filters(filters.clone());

        assert_eq!(search.object_types, default_retrieval_object_types());
        assert_eq!(default_vector_candidate_object_types(), search.object_types);
        assert_eq!(search.filters, filters);
        assert!(search.filters.has_currentness_filters());
    }

    #[test]
    fn embedding_input_keeps_raw_text_consumer_supplied() {
        let object_id = MemoryId::new_v4();
        let input = EmbeddingInput::new(
            Some(object_id),
            Some(ObjectType::Episode),
            VectorSurface::Summary,
            "episode summary",
        );

        assert_eq!(input.object_id, Some(object_id));
        assert_eq!(input.object_type, Some(ObjectType::Episode));
        assert_eq!(input.surface, VectorSurface::Summary);
        assert_eq!(input.text, "episode summary");
    }
}
