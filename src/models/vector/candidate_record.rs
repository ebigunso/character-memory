// Vector candidate query surface. Some filters are exercised by live
// adapters while deterministic tests use narrower subsets.
use std::collections::{hash_map::Entry, HashMap};

use crate::domain::{MemoryId, ObjectType};

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
#[cfg(test)]
pub(crate) struct VectorCandidateRecord {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) surface: VectorSurface,
    pub(crate) embedding: Vec<f32>,
}

#[cfg(test)]
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
            candidate.object_type.stable_rank(),
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
        .then_with(|| {
            left.object_type
                .stable_rank()
                .cmp(&right.object_type.stable_rank())
        })
        .then_with(|| left.object_id.cmp(&right.object_id))
        .then_with(|| vector_surface_rank(left.surface).cmp(&vector_surface_rank(right.surface)))
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
