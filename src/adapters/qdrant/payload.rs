// Qdrant payload mapping. These fields are denormalized candidate
// recall/filter hints, not graph authority.
use chrono::{DateTime, SecondsFormat, Utc};
use qdrant_client::qdrant::FieldType;
use serde::Serialize;

use crate::domain::schema::require_current_schema_version;
use crate::errors::CustomError;
use crate::models::vector::{VectorRecord, VectorSurface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QdrantPayloadKind {
    Keyword,
    Datetime,
    Bool,
    Float,
    Text,
}

impl QdrantPayloadKind {
    pub(crate) const fn field_type(self) -> FieldType {
        match self {
            Self::Keyword => FieldType::Keyword,
            Self::Datetime => FieldType::Datetime,
            Self::Bool => FieldType::Bool,
            Self::Float => FieldType::Float,
            Self::Text => FieldType::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QdrantPayloadField {
    ObjectId,
    GraphUri,
    ObjectType,
    DerivedType,
    EntityType,
    ThreadStatus,
    SchemaVersion,
    Surface,
    EmbeddingText,
    ContentText,
    RetentionState,
    IsCurrent,
    IsSuperseded,
    EpisodeIds,
    ObservationIds,
    ThreadIds,
    EntityIds,
    ParticipantEntityIds,
    SpeakerEntityId,
    Supersedes,
    Modality,
    SourceConversationId,
    CanonicalKey,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
    ObservedAt,
    LastTouchedAt,
    SalienceScore,
    Confidence,
    Stability,
    RawRef,
}

impl QdrantPayloadField {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ObjectId => "object_id",
            Self::GraphUri => "graph_uri",
            Self::ObjectType => "object_type",
            Self::DerivedType => "derived_type",
            Self::EntityType => "entity_type",
            Self::ThreadStatus => "thread_status",
            Self::SchemaVersion => "schema_version",
            Self::Surface => "surface",
            Self::EmbeddingText => "embedding_text",
            Self::ContentText => "content_text",
            Self::RetentionState => "retention_state",
            Self::IsCurrent => "is_current",
            Self::IsSuperseded => "is_superseded",
            Self::EpisodeIds => "episode_ids",
            Self::ObservationIds => "observation_ids",
            Self::ThreadIds => "thread_ids",
            Self::EntityIds => "entity_ids",
            Self::ParticipantEntityIds => "participant_entity_ids",
            Self::SpeakerEntityId => "speaker_entity_id",
            Self::Supersedes => "supersedes",
            Self::Modality => "modality",
            Self::SourceConversationId => "source_conversation_id",
            Self::CanonicalKey => "canonical_key",
            Self::CreatedAt => "created_at",
            Self::UpdatedAt => "updated_at",
            Self::StartedAt => "started_at",
            Self::EndedAt => "ended_at",
            Self::ObservedAt => "observed_at",
            Self::LastTouchedAt => "last_touched_at",
            Self::SalienceScore => "salience_score",
            Self::Confidence => "confidence",
            Self::Stability => "stability",
            Self::RawRef => "raw_ref",
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
            QdrantPayloadField::GraphUri,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ObjectType,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::DerivedType,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::EntityType,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ThreadStatus,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::SchemaVersion,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::Surface,
            QdrantPayloadKind::Keyword,
            false,
        ),
        schema(
            QdrantPayloadField::EmbeddingText,
            QdrantPayloadKind::Text,
            false,
        ),
        schema(
            QdrantPayloadField::ContentText,
            QdrantPayloadKind::Text,
            false,
        ),
        schema(
            QdrantPayloadField::RetentionState,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(QdrantPayloadField::IsCurrent, QdrantPayloadKind::Bool, true),
        schema(
            QdrantPayloadField::IsSuperseded,
            QdrantPayloadKind::Bool,
            true,
        ),
        schema(
            QdrantPayloadField::EpisodeIds,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ObservationIds,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ThreadIds,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::EntityIds,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::ParticipantEntityIds,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::SpeakerEntityId,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::Supersedes,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::Modality,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::SourceConversationId,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::CanonicalKey,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(
            QdrantPayloadField::CreatedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::UpdatedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::StartedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::EndedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::ObservedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::LastTouchedAt,
            QdrantPayloadKind::Datetime,
            true,
        ),
        schema(
            QdrantPayloadField::SalienceScore,
            QdrantPayloadKind::Float,
            true,
        ),
        schema(
            QdrantPayloadField::Confidence,
            QdrantPayloadKind::Float,
            true,
        ),
        schema(
            QdrantPayloadField::Stability,
            QdrantPayloadKind::Keyword,
            true,
        ),
        schema(QdrantPayloadField::RawRef, QdrantPayloadKind::Keyword, true),
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
#[cfg(test)]
pub(crate) const GRAPH_URI_FIELD: &str = QdrantPayloadField::GraphUri.name();
pub(crate) const OBJECT_TYPE_FIELD: &str = QdrantPayloadField::ObjectType.name();
#[cfg(test)]
pub(crate) const DERIVED_TYPE_FIELD: &str = QdrantPayloadField::DerivedType.name();
#[cfg(test)]
pub(crate) const SCHEMA_VERSION_FIELD: &str = QdrantPayloadField::SchemaVersion.name();
pub(crate) const SURFACE_FIELD: &str = QdrantPayloadField::Surface.name();
#[cfg(test)]
pub(crate) const RETENTION_STATE_FIELD: &str = QdrantPayloadField::RetentionState.name();
#[cfg(test)]
pub(crate) const IS_CURRENT_FIELD: &str = QdrantPayloadField::IsCurrent.name();
#[cfg(test)]
pub(crate) const IS_SUPERSEDED_FIELD: &str = QdrantPayloadField::IsSuperseded.name();
#[cfg(test)]
pub(crate) const EPISODE_IDS_FIELD: &str = QdrantPayloadField::EpisodeIds.name();
#[cfg(test)]
pub(crate) const OBSERVATION_IDS_FIELD: &str = QdrantPayloadField::ObservationIds.name();
#[cfg(test)]
pub(crate) const THREAD_IDS_FIELD: &str = QdrantPayloadField::ThreadIds.name();
#[cfg(test)]
pub(crate) const ENTITY_IDS_FIELD: &str = QdrantPayloadField::EntityIds.name();
#[cfg(test)]
pub(crate) const SUPERSEDES_FIELD: &str = QdrantPayloadField::Supersedes.name();
#[cfg(test)]
pub(crate) const MODALITY_FIELD: &str = QdrantPayloadField::Modality.name();
#[cfg(test)]
pub(crate) const CREATED_AT_FIELD: &str = QdrantPayloadField::CreatedAt.name();
#[cfg(test)]
pub(crate) const UPDATED_AT_FIELD: &str = QdrantPayloadField::UpdatedAt.name();
#[cfg(test)]
pub(crate) const OBSERVED_AT_FIELD: &str = QdrantPayloadField::ObservedAt.name();
#[cfg(test)]
pub(crate) const LAST_TOUCHED_AT_FIELD: &str = QdrantPayloadField::LastTouchedAt.name();
#[cfg(test)]
pub(crate) const SALIENCE_SCORE_FIELD: &str = QdrantPayloadField::SalienceScore.name();
#[cfg(test)]
pub(crate) const CONFIDENCE_FIELD: &str = QdrantPayloadField::Confidence.name();
#[cfg(test)]
pub(crate) const STABILITY_FIELD: &str = QdrantPayloadField::Stability.name();
#[cfg(test)]
pub(crate) const RAW_REF_FIELD: &str = QdrantPayloadField::RawRef.name();

#[cfg(test)]
pub(crate) const GRAPH_AUTHORITY_NOTE: &str =
    "Qdrant relationship ID fields are denormalized filter hints only; GraphAuthorityStore remains authoritative for relationships, provenance, lifecycle, currentness, and graph expansion.";
#[cfg(test)]
pub(crate) const EMBEDDING_TEXT_FIELD: &str = QdrantPayloadField::EmbeddingText.name();
#[cfg(test)]
pub(crate) const CONTENT_TEXT_FIELD: &str = QdrantPayloadField::ContentText.name();

pub(crate) fn qdrant_payload_map(
    record: &VectorRecord,
) -> Result<serde_json::Map<String, serde_json::Value>, CustomError> {
    require_current_schema_version(&record.schema_version, "Qdrant payload mapping")?;

    let hints = &record.payload_hints;
    let relationships = &record.relationship_hints;
    let mut payload = serde_json::Map::new();
    insert_value(
        &mut payload,
        QdrantPayloadField::ObjectId,
        record.object_id.to_string(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::GraphUri,
        record.graph_uri.clone(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::ObjectType,
        enum_value(record.object_type),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::SchemaVersion,
        record.schema_version.clone(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::Surface,
        vector_surface(record.surface),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::EmbeddingText,
        record.embedding_text.clone(),
    )?;
    insert_value(
        &mut payload,
        QdrantPayloadField::ContentText,
        record.content_text.clone(),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::DerivedType,
        hints.derived_type.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::EntityType,
        hints.entity_type.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::ThreadStatus,
        hints.thread_status.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::RetentionState,
        record.retention_state.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::IsCurrent,
        record.is_current,
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::IsSuperseded,
        hints
            .is_superseded
            .or_else(|| record.is_current.map(|value| !value)),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::EpisodeIds,
        ids(&relationships.episode_ids),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::ObservationIds,
        ids(&relationships.observation_ids),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::ThreadIds,
        ids(&relationships.thread_ids),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::EntityIds,
        ids(&relationships.entity_ids),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::ParticipantEntityIds,
        ids(&relationships.participant_entity_ids),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::SpeakerEntityId,
        relationships.speaker_entity_id.map(|id| id.to_string()),
    )?;
    insert_non_empty(
        &mut payload,
        QdrantPayloadField::Supersedes,
        ids(&relationships.supersedes),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::Modality,
        hints.modality.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::SourceConversationId,
        hints.source_conversation_id.clone(),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::CanonicalKey,
        hints.canonical_key.clone(),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::CreatedAt,
        hints.created_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::UpdatedAt,
        hints.updated_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::StartedAt,
        hints.started_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::EndedAt,
        hints.ended_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::ObservedAt,
        hints.observed_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::LastTouchedAt,
        hints.last_touched_at.map(timestamp),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::SalienceScore,
        hints.salience_score,
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::Confidence,
        hints.confidence,
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::Stability,
        hints.stability.map(enum_value),
    )?;
    insert_optional(
        &mut payload,
        QdrantPayloadField::RawRef,
        record.raw_ref.clone(),
    )?;
    Ok(payload)
}

fn insert_value(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    field: QdrantPayloadField,
    value: impl Serialize,
) -> Result<(), CustomError> {
    let field_schema = QdrantPayloadSchema::field_schema(field);
    payload.insert(
        field_schema.field.name().to_owned(),
        serde_json::to_value(value)?,
    );
    Ok(())
}

fn insert_optional(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    field: QdrantPayloadField,
    value: Option<impl Serialize>,
) -> Result<(), CustomError> {
    if let Some(value) = value {
        insert_value(payload, field, value)?;
    }
    Ok(())
}

fn insert_non_empty<T: Serialize>(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    field: QdrantPayloadField,
    values: Vec<T>,
) -> Result<(), CustomError> {
    if !values.is_empty() {
        insert_value(payload, field, values)?;
    }
    Ok(())
}

fn ids(ids: &[uuid::Uuid]) -> Vec<String> {
    ids.iter().map(ToString::to_string).collect()
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn enum_value(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

fn vector_surface(surface: VectorSurface) -> &'static str {
    match surface {
        VectorSurface::Summary => "summary",
        VectorSurface::Text => "text",
        VectorSurface::Name => "name",
        VectorSurface::DerivedText => "derived_text",
        VectorSurface::Query => "query",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        graph_uri, DerivedType, EntityType, Modality, ObjectType, RetentionState, Stability,
        ThreadStatus, DEFAULT_SCHEMA_VERSION,
    };
    use crate::models::vector::{
        VectorPayloadHints, VectorRecord, VectorRelationshipHints, VectorSurface,
    };
    use chrono::TimeZone;
    use qdrant_client::qdrant::FieldType;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn payload_maps_identity_text_lifecycle_time_and_scores() {
        let object_id = id(40);
        let record = derived_memory_record(object_id);

        let payload = qdrant_payload_map(&record).expect("payload maps");

        assert_eq!(payload[OBJECT_ID_FIELD], json!(object_id.to_string()));
        assert_eq!(
            payload[GRAPH_URI_FIELD],
            json!(graph_uri(ObjectType::DerivedMemory, object_id))
        );
        assert_eq!(payload[OBJECT_TYPE_FIELD], json!("derived_memory"));
        assert!(payload.get("record_type").is_none());
        assert_eq!(payload[DERIVED_TYPE_FIELD], json!("reflection"));
        assert_eq!(payload[SCHEMA_VERSION_FIELD], json!(DEFAULT_SCHEMA_VERSION));
        assert_eq!(
            payload[EMBEDDING_TEXT_FIELD],
            json!("Reflection: Keep Qdrant filter-only.")
        );
        assert_eq!(
            payload[CONTENT_TEXT_FIELD],
            json!("Keep Qdrant filter-only.")
        );
        assert_eq!(payload[RETENTION_STATE_FIELD], json!("active"));
        assert_eq!(payload[IS_CURRENT_FIELD], json!(false));
        assert_eq!(payload[IS_SUPERSEDED_FIELD], json!(true));
        assert_eq!(payload[CREATED_AT_FIELD], json!("2026-04-28T12:00:00Z"));
        assert_eq!(payload[UPDATED_AT_FIELD], json!("2026-04-28T12:00:00Z"));
        assert_float(&payload[SALIENCE_SCORE_FIELD], 0.91);
        assert_float(&payload[CONFIDENCE_FIELD], 0.82);
        assert_eq!(payload[STABILITY_FIELD], json!("medium"));
    }

    #[test]
    fn payload_relationship_ids_are_filter_hints_not_graph_authority() {
        let payload = qdrant_payload_map(&derived_memory_record(id(40))).expect("payload maps");

        assert_eq!(payload[EPISODE_IDS_FIELD], json!([id(10).to_string()]));
        assert_eq!(payload[OBSERVATION_IDS_FIELD], json!([id(20).to_string()]));
        assert_eq!(payload[THREAD_IDS_FIELD], json!([id(30).to_string()]));
        assert_eq!(payload[ENTITY_IDS_FIELD], json!([id(1).to_string()]));
        assert_eq!(payload[SUPERSEDES_FIELD], json!([id(99).to_string()]));
        assert!(GRAPH_AUTHORITY_NOTE.contains("GraphAuthorityStore"));
        assert!(GRAPH_AUTHORITY_NOTE.contains("filter hints"));
    }

    #[test]
    fn payload_preserves_schema_version_field() {
        let payload = qdrant_payload_map(&derived_memory_record(id(40))).expect("payload maps");

        assert_eq!(payload[SCHEMA_VERSION_FIELD], json!(DEFAULT_SCHEMA_VERSION));
    }

    #[test]
    fn payload_preserves_raw_ref_without_full_raw_transcript_field() {
        let record = VectorRecord::new(
            id(10),
            ObjectType::Episode,
            graph_uri(ObjectType::Episode, id(10)),
            VectorSurface::Summary,
            "Episode summary: Short summary.",
            "Short summary.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            None,
            VectorRelationshipHints::default(),
            Some("raw://conversation/chat_123#turn_42".to_owned()),
        );

        let payload = qdrant_payload_map(&record).expect("payload maps");

        assert_eq!(
            payload[RAW_REF_FIELD],
            json!("raw://conversation/chat_123#turn_42")
        );
        assert!(payload.get("raw_transcript").is_none());
        assert!(payload.get("raw_text").is_none());
        assert!(payload.get("transcript").is_none());
        assert!(payload.get("source_transcript").is_none());
    }

    #[test]
    fn payload_mapping_rejects_unsupported_schema_versions() {
        let mut record = derived_memory_record(id(40));
        record.schema_version = "future_schema".to_owned();

        let error = qdrant_payload_map(&record).expect_err("unsupported schema fails");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion {
                context: "Qdrant payload mapping",
                ..
            }
        ));
    }

    #[test]
    fn schema_manifest_drives_payload_keys_and_indexes() {
        let payloads = [
            qdrant_payload_map(&derived_memory_record(id(40))).expect("derived payload maps"),
            qdrant_payload_map(&fully_populated_record()).expect("complete payload maps"),
        ];
        let schema_names = QdrantPayloadSchema::FIELDS
            .iter()
            .map(|field| field.field.name())
            .collect::<std::collections::HashSet<_>>();
        let emitted_names = payloads
            .iter()
            .flat_map(|payload| payload.keys().map(String::as_str))
            .collect::<std::collections::HashSet<_>>();
        let indexed_fields = QdrantPayloadSchema::indexed_fields().collect::<Vec<_>>();
        let indexed_names = indexed_fields
            .iter()
            .map(|field| field.field.name())
            .collect::<Vec<_>>();

        assert_eq!(schema_names.len(), QdrantPayloadSchema::FIELDS.len());
        assert_eq!(emitted_names, schema_names);
        assert!(!schema_names.contains("record_type"));

        for expected in [
            OBJECT_TYPE_FIELD,
            DERIVED_TYPE_FIELD,
            SCHEMA_VERSION_FIELD,
            ENTITY_IDS_FIELD,
            THREAD_IDS_FIELD,
            EPISODE_IDS_FIELD,
            MODALITY_FIELD,
            CREATED_AT_FIELD,
            OBSERVED_AT_FIELD,
            LAST_TOUCHED_AT_FIELD,
            IS_CURRENT_FIELD,
            IS_SUPERSEDED_FIELD,
            RETENTION_STATE_FIELD,
            SALIENCE_SCORE_FIELD,
            CONFIDENCE_FIELD,
            RAW_REF_FIELD,
        ] {
            assert!(
                indexed_names.contains(&expected),
                "missing index field {expected}"
            );
        }

        assert_eq!(field_type(CREATED_AT_FIELD), FieldType::Datetime);
        assert_eq!(field_type(IS_CURRENT_FIELD), FieldType::Bool);
        assert_eq!(field_type(SALIENCE_SCORE_FIELD), FieldType::Float);
        assert!(!indexed_names.contains(&EMBEDDING_TEXT_FIELD));
        assert!(!indexed_names.contains(&CONTENT_TEXT_FIELD));
    }

    fn derived_memory_record(object_id: Uuid) -> VectorRecord {
        VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Keep Qdrant filter-only.",
            "Keep Qdrant filter-only.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            Some(false),
            VectorRelationshipHints {
                episode_ids: vec![id(10)],
                observation_ids: vec![id(20)],
                thread_ids: vec![id(30)],
                entity_ids: vec![id(1)],
                supersedes: vec![id(99)],
                ..VectorRelationshipHints::default()
            },
            None,
        )
        .with_payload_hints(VectorPayloadHints {
            derived_type: Some(DerivedType::Reflection),
            created_at: Some(timestamp_fixture()),
            updated_at: Some(timestamp_fixture()),
            salience_score: Some(0.91),
            confidence: Some(0.82),
            stability: Some(Stability::Medium),
            is_superseded: Some(true),
            ..VectorPayloadHints::default()
        })
    }

    fn fully_populated_record() -> VectorRecord {
        VectorRecord::new(
            id(200),
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, id(200)),
            VectorSurface::DerivedText,
            "Fully populated embedding text",
            "Fully populated content text",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            Some(false),
            VectorRelationshipHints {
                episode_ids: vec![id(201)],
                observation_ids: vec![id(202)],
                thread_ids: vec![id(203)],
                entity_ids: vec![id(204)],
                participant_entity_ids: vec![id(205)],
                speaker_entity_id: Some(id(206)),
                supersedes: vec![id(207)],
            },
            Some("raw://fully-populated".to_owned()),
        )
        .with_payload_hints(VectorPayloadHints {
            modality: Some(Modality::Chat),
            derived_type: Some(DerivedType::Reflection),
            entity_type: Some(EntityType::Concept),
            thread_status: Some(ThreadStatus::Active),
            source_conversation_id: Some("conversation-1".to_owned()),
            canonical_key: Some("canonical-1".to_owned()),
            created_at: Some(timestamp_fixture()),
            updated_at: Some(timestamp_fixture()),
            started_at: Some(timestamp_fixture()),
            ended_at: Some(timestamp_fixture()),
            observed_at: Some(timestamp_fixture()),
            last_touched_at: Some(timestamp_fixture()),
            salience_score: Some(0.91),
            confidence: Some(0.82),
            stability: Some(Stability::Medium),
            is_superseded: Some(true),
        })
    }

    fn field_type(name: &str) -> FieldType {
        QdrantPayloadSchema::FIELDS
            .iter()
            .find(|field| field.field.name() == name)
            .map(|field| field.kind.field_type())
            .expect("field exists")
    }

    fn assert_float(value: &serde_json::Value, expected: f64) {
        let actual = value.as_f64().expect("numeric payload value");
        assert!((actual - expected).abs() < 0.000_001);
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0000 + value)
    }

    fn timestamp_fixture() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap()
    }
}
