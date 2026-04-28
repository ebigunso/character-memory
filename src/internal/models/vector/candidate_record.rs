// Transitional contract model surface: some fields/builders are reserved
// for adapter and pipeline chunks. Remove once those chunks consume the record
// types directly, or prune unused members.
#![allow(dead_code)]

use chrono::{DateTime, Utc};

use crate::api::types::{MemoryId, ObjectType, RetentionState};

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

    pub(crate) fn with_default_object_types(mut self) -> Self {
        self.object_types = default_vector_candidate_object_types();
        self
    }

    pub(crate) fn with_filters(mut self, filters: VectorCandidateFilters) -> Self {
        self.filters = filters;
        self
    }
}

pub(crate) fn default_vector_candidate_object_types() -> Vec<ObjectType> {
    vec![
        ObjectType::Episode,
        ObjectType::Observation,
        ObjectType::DerivedMemory,
        ObjectType::MemoryThread,
        ObjectType::Entity,
    ]
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

    pub(crate) fn with_retention_states(mut self, retention_states: Vec<RetentionState>) -> Self {
        self.retention_states = retention_states;
        self
    }

    pub(crate) fn current_only(mut self) -> Self {
        self.is_current = Some(true);
        self.is_superseded = Some(false);
        self
    }

    pub(crate) fn with_thread_ids(mut self, thread_ids: Vec<MemoryId>) -> Self {
        self.thread_ids = thread_ids;
        self
    }

    pub(crate) fn with_entity_ids(mut self, entity_ids: Vec<MemoryId>) -> Self {
        self.entity_ids = entity_ids;
        self
    }

    pub(crate) fn with_episode_ids(mut self, episode_ids: Vec<MemoryId>) -> Self {
        self.episode_ids = episode_ids;
        self
    }

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

        assert_eq!(search.object_types, default_vector_candidate_object_types());
        assert_eq!(search.filters, filters);
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
