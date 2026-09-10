// Provider-neutral five-field vector record. Read-out content and graph state
// remain in graph authority storage.
use crate::domain::{MemoryId, ObjectType, VectorSurface};

use super::EmbeddingInput;

#[derive(Debug, Clone, Copy)]
pub(crate) struct VectorRecordEmbedding<'a> {
    pub(crate) record: &'a VectorRecord,
    pub(crate) embedding: &'a [f32],
}

impl<'a> VectorRecordEmbedding<'a> {
    pub(crate) fn new(record: &'a VectorRecord, embedding: &'a [f32]) -> Self {
        Self { record, embedding }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorRecord {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) surface: VectorSurface,
    pub(crate) schema_version: String,
    pub(crate) embedding_text: String,
}

impl VectorRecord {
    pub(crate) fn new(
        object_id: MemoryId,
        object_type: ObjectType,
        surface: VectorSurface,
        schema_version: impl Into<String>,
        embedding_text: impl Into<String>,
    ) -> Self {
        Self {
            object_id,
            object_type,
            surface,
            schema_version: schema_version.into(),
            embedding_text: embedding_text.into(),
        }
    }

    pub(crate) fn embedding_input(&self) -> EmbeddingInput {
        EmbeddingInput::new(
            Some(self.object_id),
            Some(self.object_type),
            self.surface,
            self.embedding_text.clone(),
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
    use crate::domain::DEFAULT_SCHEMA_VERSION;

    #[test]
    fn vector_record_converts_to_embedding_input_without_payload_metadata() {
        let object_id = MemoryId::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            VectorSurface::Summary,
            DEFAULT_SCHEMA_VERSION,
            "Episode summary: Discussed contract tests.",
        );

        let input = record.embedding_input();

        assert_eq!(input.object_id, Some(object_id));
        assert_eq!(input.object_type, Some(ObjectType::Episode));
        assert_eq!(input.surface, VectorSurface::Summary);
        assert_eq!(input.text, "Episode summary: Discussed contract tests.");
        assert!(!input.text.contains(&object_id.to_string()));
        assert!(!input.text.contains(DEFAULT_SCHEMA_VERSION));
    }
}
