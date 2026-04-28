// Transitional contract model surface: some fields/builders are reserved
// for adapter and pipeline chunks. Remove once those chunks consume the record
// types directly, or prune unused members.
#![allow(dead_code)]

use crate::api::types::{MemoryId, ObjectType};

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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateSearch {
    pub(crate) query_embedding: Vec<f32>,
    pub(crate) limit: usize,
    pub(crate) object_types: Vec<ObjectType>,
}

impl VectorCandidateSearch {
    pub(crate) fn new(query_embedding: Vec<f32>, limit: usize) -> Self {
        Self {
            query_embedding,
            limit,
            object_types: Vec::new(),
        }
    }

    pub(crate) fn with_object_types(mut self, object_types: Vec<ObjectType>) -> Self {
        self.object_types = object_types;
        self
    }
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
