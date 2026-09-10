// Qdrant stores only vector identity, provenance, version, and embedding input.
// Read-out content and graph state are hydrated from graph authority storage.
use qdrant_client::qdrant::FieldType;

use crate::domain::schema::require_current_schema_version;
use crate::errors::CustomError;
use crate::models::vector::VectorRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QdrantPayloadKind {
    Keyword,
    Text,
}

impl QdrantPayloadKind {
    pub(crate) const fn field_type(self) -> FieldType {
        match self {
            Self::Keyword => FieldType::Keyword,
            Self::Text => FieldType::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QdrantPayloadField {
    ObjectId,
    ObjectType,
    Surface,
    SchemaVersion,
    EmbeddingText,
}

impl QdrantPayloadField {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ObjectId => "object_id",
            Self::ObjectType => "object_type",
            Self::Surface => "surface",
            Self::SchemaVersion => "schema_version",
            Self::EmbeddingText => "embedding_text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QdrantPayloadFieldSchema {
    pub(crate) field: QdrantPayloadField,
    pub(crate) kind: QdrantPayloadKind,
    pub(crate) indexed: bool,
}

pub(crate) struct QdrantPayloadSchema;

impl QdrantPayloadSchema {
    pub(crate) const FIELDS: &[QdrantPayloadFieldSchema] = &[
        schema(
            QdrantPayloadField::ObjectId,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ObjectType,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::Surface,
            QdrantPayloadKind::Keyword,
            false,
        ),
        schema(
            QdrantPayloadField::SchemaVersion,
            QdrantPayloadKind::Keyword,
            false,
        ),
        schema(
            QdrantPayloadField::EmbeddingText,
            QdrantPayloadKind::Text,
            false,
        ),
    ];

    pub(crate) fn indexed_fields() -> impl Iterator<Item = &'static QdrantPayloadFieldSchema> {
        Self::FIELDS.iter().filter(|field| field.indexed)
    }

    fn field_schema(field: QdrantPayloadField) -> &'static QdrantPayloadFieldSchema {
        Self::FIELDS
            .iter()
            .find(|schema| schema.field == field)
            .expect("every writable Qdrant payload field must be declared in the schema")
    }
}

const fn schema(
    field: QdrantPayloadField,
    kind: QdrantPayloadKind,
    indexed: bool,
) -> QdrantPayloadFieldSchema {
    QdrantPayloadFieldSchema {
        field,
        kind,
        indexed,
    }
}

pub(crate) const OBJECT_ID_FIELD: &str = QdrantPayloadField::ObjectId.name();
pub(crate) const OBJECT_TYPE_FIELD: &str = QdrantPayloadField::ObjectType.name();
pub(crate) const SURFACE_FIELD: &str = QdrantPayloadField::Surface.name();

pub(crate) fn qdrant_payload_map(
    record: &VectorRecord,
) -> Result<serde_json::Map<String, serde_json::Value>, CustomError> {
    require_current_schema_version(&record.schema_version, "Qdrant payload mapping")?;

    let mut payload = serde_json::Map::new();
    insert_value(
        &mut payload,
        QdrantPayloadField::ObjectId,
        record.object_id.to_string(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::ObjectType,
        record.object_type.to_string(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::Surface,
        record.surface.to_string(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::SchemaVersion,
        record.schema_version.clone(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::EmbeddingText,
        record.embedding_text.clone(),
    )?;
    Ok(payload)
}

fn insert_value(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    field: QdrantPayloadField,
    value: impl serde::Serialize,
) -> Result<(), CustomError> {
    let schema = QdrantPayloadSchema::field_schema(field);
    let previous = payload.insert(field.name().to_owned(), serde_json::to_value(value)?);
    debug_assert!(previous.is_none());
    debug_assert_eq!(schema.field, field);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MemoryId, ObjectType, VectorSurface, DEFAULT_SCHEMA_VERSION};
    use crate::models::vector::VectorRecord;
    use serde_json::json;

    #[test]
    fn payload_maps_exact_five_field_record() {
        let object_id = MemoryId::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            VectorSurface::DerivedText,
            DEFAULT_SCHEMA_VERSION,
            "Reflection: Prefer the smallest durable contract.",
        );

        let payload = qdrant_payload_map(&record).expect("payload maps");

        assert_eq!(payload.len(), 5);
        assert_eq!(
            payload[QdrantPayloadField::ObjectId.name()],
            json!(object_id)
        );
        assert_eq!(
            payload[QdrantPayloadField::ObjectType.name()],
            json!("derived_memory")
        );
        assert_eq!(
            payload[QdrantPayloadField::Surface.name()],
            json!("derived_text")
        );
        assert_eq!(
            payload[QdrantPayloadField::SchemaVersion.name()],
            json!(DEFAULT_SCHEMA_VERSION)
        );
        assert_eq!(
            payload[QdrantPayloadField::EmbeddingText.name()],
            json!("Reflection: Prefer the smallest durable contract.")
        );
    }

    #[test]
    fn manifest_declares_exact_record_and_only_identity_indexes() {
        assert_eq!(
            QdrantPayloadSchema::FIELDS,
            &[
                schema(
                    QdrantPayloadField::ObjectId,
                    QdrantPayloadKind::Keyword,
                    true
                ),
                schema(
                    QdrantPayloadField::ObjectType,
                    QdrantPayloadKind::Keyword,
                    true
                ),
                schema(
                    QdrantPayloadField::Surface,
                    QdrantPayloadKind::Keyword,
                    false
                ),
                schema(
                    QdrantPayloadField::SchemaVersion,
                    QdrantPayloadKind::Keyword,
                    false
                ),
                schema(
                    QdrantPayloadField::EmbeddingText,
                    QdrantPayloadKind::Text,
                    false
                ),
            ]
        );
        assert_eq!(
            QdrantPayloadSchema::indexed_fields()
                .map(|field| field.field)
                .collect::<Vec<_>>(),
            vec![QdrantPayloadField::ObjectId, QdrantPayloadField::ObjectType,]
        );
    }

    #[test]
    fn payload_rejects_non_current_schema_version() {
        let record = VectorRecord::new(
            MemoryId::new_v4(),
            ObjectType::Episode,
            VectorSurface::Summary,
            "future_schema",
            "Episode summary",
        );

        let error = qdrant_payload_map(&record).expect_err("future schema is rejected");
        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion {
                context: "Qdrant payload mapping",
                expected: DEFAULT_SCHEMA_VERSION,
                actual,
            } if actual == "future_schema"
        ));
    }
}
