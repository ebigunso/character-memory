// Provider-neutral vector record surface. Payload hints remain
// denormalized recall/filter hints; graph state stays authoritative.
#![allow(dead_code)]

use chrono::{DateTime, Utc};

use crate::api::types::{
    DerivedType, EntityType, MemoryId, Modality, ObjectType, RetentionState, Stability,
    ThreadStatus,
};

use super::{EmbeddingInput, VectorCandidateRecord, VectorSurface};

#[derive(Debug, Clone, Copy)]
pub(crate) struct VectorRecordEmbedding<'a> {
    pub(crate) record: &'a VectorRecord,
    pub(crate) embedding: &'a [f32],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateDiagnosticRecord {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) graph_uri: String,
    pub(crate) surface: VectorSurface,
    pub(crate) schema_version: String,
    pub(crate) retention_state: Option<RetentionState>,
    pub(crate) is_current: Option<bool>,
    pub(crate) is_superseded: Option<bool>,
}

impl VectorCandidateDiagnosticRecord {
    pub(crate) fn from_vector_record(record: &VectorRecord) -> Self {
        Self {
            object_id: record.object_id,
            object_type: record.object_type,
            graph_uri: record.graph_uri.clone(),
            surface: record.surface,
            schema_version: record.schema_version.clone(),
            retention_state: record.retention_state,
            is_current: record.is_current,
            is_superseded: record
                .payload_hints
                .is_superseded
                .or_else(|| record.is_current.map(|value| !value)),
        }
    }
}

impl<'a> VectorRecordEmbedding<'a> {
    pub(crate) fn new(record: &'a VectorRecord, embedding: &'a [f32]) -> Self {
        Self { record, embedding }
    }

    pub(crate) fn to_candidate_record(self) -> VectorCandidateRecord {
        self.record.to_candidate_record(self.embedding.to_vec())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct VectorRelationshipHints {
    pub(crate) episode_ids: Vec<MemoryId>,
    pub(crate) observation_ids: Vec<MemoryId>,
    pub(crate) thread_ids: Vec<MemoryId>,
    pub(crate) entity_ids: Vec<MemoryId>,
    pub(crate) participant_entity_ids: Vec<MemoryId>,
    pub(crate) speaker_entity_id: Option<MemoryId>,
    pub(crate) supersedes: Vec<MemoryId>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct VectorPayloadHints {
    pub(crate) modality: Option<Modality>,
    pub(crate) derived_type: Option<DerivedType>,
    pub(crate) entity_type: Option<EntityType>,
    pub(crate) thread_status: Option<ThreadStatus>,
    pub(crate) source_conversation_id: Option<String>,
    pub(crate) canonical_key: Option<String>,
    pub(crate) created_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) ended_at: Option<DateTime<Utc>>,
    pub(crate) observed_at: Option<DateTime<Utc>>,
    pub(crate) last_touched_at: Option<DateTime<Utc>>,
    pub(crate) salience_score: Option<f32>,
    pub(crate) confidence: Option<f32>,
    pub(crate) stability: Option<Stability>,
    pub(crate) is_superseded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorRecord {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) graph_uri: String,
    pub(crate) surface: VectorSurface,
    pub(crate) embedding_text: String,
    pub(crate) content_text: String,
    pub(crate) schema_version: String,
    pub(crate) retention_state: Option<RetentionState>,
    pub(crate) is_current: Option<bool>,
    pub(crate) relationship_hints: VectorRelationshipHints,
    pub(crate) payload_hints: VectorPayloadHints,
    pub(crate) raw_ref: Option<String>,
}

impl VectorRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        object_id: MemoryId,
        object_type: ObjectType,
        graph_uri: impl Into<String>,
        surface: VectorSurface,
        embedding_text: impl Into<String>,
        content_text: impl Into<String>,
        schema_version: impl Into<String>,
        retention_state: Option<RetentionState>,
        is_current: Option<bool>,
        relationship_hints: VectorRelationshipHints,
        raw_ref: Option<String>,
    ) -> Self {
        Self {
            object_id,
            object_type,
            graph_uri: graph_uri.into(),
            surface,
            embedding_text: embedding_text.into(),
            content_text: content_text.into(),
            schema_version: schema_version.into(),
            retention_state,
            is_current,
            relationship_hints,
            payload_hints: VectorPayloadHints::default(),
            raw_ref,
        }
    }

    pub(crate) fn with_payload_hints(mut self, payload_hints: VectorPayloadHints) -> Self {
        self.payload_hints = payload_hints;
        self
    }

    pub(crate) fn embedding_input(&self) -> EmbeddingInput {
        EmbeddingInput::new(
            Some(self.object_id),
            Some(self.object_type),
            self.surface,
            self.embedding_text.clone(),
        )
    }

    pub(crate) fn to_candidate_record(&self, embedding: Vec<f32>) -> VectorCandidateRecord {
        VectorCandidateRecord::new(self.object_id, self.object_type, self.surface, embedding)
            .with_filter_hints(
                self.retention_state,
                self.is_current,
                self.relationship_hints.clone(),
                self.payload_hints.clone(),
            )
    }
}

impl From<&VectorRecord> for EmbeddingInput {
    fn from(record: &VectorRecord) -> Self {
        record.embedding_input()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{graph_uri, DEFAULT_SCHEMA_VERSION};

    #[test]
    fn vector_record_converts_to_embedding_input_without_payload_metadata() {
        let object_id = MemoryId::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            graph_uri(ObjectType::Episode, object_id),
            VectorSurface::Summary,
            "Episode summary: Discussed contract tests.",
            "Discussed contract tests.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            None,
            VectorRelationshipHints::default(),
            Some("file:raw/ref.txt".to_owned()),
        );

        let input = record.embedding_input();

        assert_eq!(input.object_id, Some(object_id));
        assert_eq!(input.object_type, Some(ObjectType::Episode));
        assert_eq!(input.surface, VectorSurface::Summary);
        assert_eq!(input.text, "Episode summary: Discussed contract tests.");
        assert!(!input.text.contains(&object_id.to_string()));
        assert!(!input.text.contains(&record.graph_uri));
        assert!(!input.text.contains(DEFAULT_SCHEMA_VERSION));
        assert!(!input.text.contains("file:raw/ref.txt"));
    }

    #[test]
    fn vector_record_converts_to_existing_candidate_contract_with_embedding() {
        let object_id = MemoryId::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::Observation,
            graph_uri(ObjectType::Observation, object_id),
            VectorSurface::Text,
            "Observation excerpt: Use deterministic fakes.",
            "Use deterministic fakes.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            None,
            VectorRelationshipHints::default(),
            None,
        );

        let candidate = record.to_candidate_record(vec![0.1, 0.2]);

        assert_eq!(candidate.object_id, object_id);
        assert_eq!(candidate.object_type, ObjectType::Observation);
        assert_eq!(candidate.surface, VectorSurface::Text);
        assert_eq!(candidate.embedding, vec![0.1, 0.2]);
        assert_eq!(candidate.retention_state, Some(RetentionState::Active));
    }
}
