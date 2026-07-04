// Qdrant payload mapping. These fields are denormalized candidate
// recall/filter hints, not graph authority.
#![allow(dead_code)]

use chrono::{DateTime, SecondsFormat, Utc};
use qdrant_client::qdrant::FieldType;
use serde::Serialize;

use crate::domain::schema::require_current_schema_version;
use crate::errors::CustomError;
use crate::models::vector::{VectorRecord, VectorSurface};

pub(crate) const GRAPH_AUTHORITY_NOTE: &str =
    "Qdrant relationship ID fields are denormalized filter hints only; GraphAuthorityStore remains authoritative for relationships, provenance, lifecycle, currentness, and graph expansion.";

pub(crate) const OBJECT_ID_FIELD: &str = "object_id";
pub(crate) const GRAPH_URI_FIELD: &str = "graph_uri";
pub(crate) const OBJECT_TYPE_FIELD: &str = "object_type";
pub(crate) const RECORD_TYPE_FIELD: &str = "record_type";
pub(crate) const DERIVED_TYPE_FIELD: &str = "derived_type";
pub(crate) const ENTITY_TYPE_FIELD: &str = "entity_type";
pub(crate) const THREAD_STATUS_FIELD: &str = "thread_status";
pub(crate) const SCHEMA_VERSION_FIELD: &str = "schema_version";
pub(crate) const EMBEDDING_TEXT_FIELD: &str = "embedding_text";
pub(crate) const CONTENT_TEXT_FIELD: &str = "content_text";
pub(crate) const SURFACE_FIELD: &str = "surface";
pub(crate) const RETENTION_STATE_FIELD: &str = "retention_state";
pub(crate) const IS_CURRENT_FIELD: &str = "is_current";
pub(crate) const IS_SUPERSEDED_FIELD: &str = "is_superseded";
pub(crate) const EPISODE_IDS_FIELD: &str = "episode_ids";
pub(crate) const OBSERVATION_IDS_FIELD: &str = "observation_ids";
pub(crate) const THREAD_IDS_FIELD: &str = "thread_ids";
pub(crate) const ENTITY_IDS_FIELD: &str = "entity_ids";
pub(crate) const PARTICIPANT_ENTITY_IDS_FIELD: &str = "participant_entity_ids";
pub(crate) const SPEAKER_ENTITY_ID_FIELD: &str = "speaker_entity_id";
pub(crate) const SUPERSEDES_FIELD: &str = "supersedes";
pub(crate) const MODALITY_FIELD: &str = "modality";
pub(crate) const SOURCE_CONVERSATION_ID_FIELD: &str = "source_conversation_id";
pub(crate) const CANONICAL_KEY_FIELD: &str = "canonical_key";
pub(crate) const CREATED_AT_FIELD: &str = "created_at";
pub(crate) const UPDATED_AT_FIELD: &str = "updated_at";
pub(crate) const STARTED_AT_FIELD: &str = "started_at";
pub(crate) const ENDED_AT_FIELD: &str = "ended_at";
pub(crate) const OBSERVED_AT_FIELD: &str = "observed_at";
pub(crate) const LAST_TOUCHED_AT_FIELD: &str = "last_touched_at";
pub(crate) const SALIENCE_SCORE_FIELD: &str = "salience_score";
pub(crate) const CONFIDENCE_FIELD: &str = "confidence";
pub(crate) const STABILITY_FIELD: &str = "stability";
pub(crate) const RAW_REF_FIELD: &str = "raw_ref";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QdrantPayloadIndexField {
    pub(crate) name: &'static str,
    pub(crate) field_type: FieldType,
    pub(crate) purpose: &'static str,
}

pub(crate) fn qdrant_payload_map(
    record: &VectorRecord,
) -> Result<serde_json::Map<String, serde_json::Value>, CustomError> {
    require_current_schema_version(&record.schema_version, "Qdrant payload mapping")?;

    let payload = QdrantVectorPayload::from(record);
    serde_json::to_value(payload)?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            CustomError::DatabaseError("Failed to convert Qdrant payload to object".to_owned())
        })
}

pub(crate) fn qdrant_payload_index_fields() -> Vec<QdrantPayloadIndexField> {
    vec![
        keyword(OBJECT_ID_FIELD, "stable vector-to-graph join id"),
        keyword(GRAPH_URI_FIELD, "stable graph resource pointer"),
        keyword(OBJECT_TYPE_FIELD, "canonical memory object filtering"),
        keyword(RECORD_TYPE_FIELD, "indexed vector record kind filtering"),
        keyword(DERIVED_TYPE_FIELD, "derived memory subtype filtering"),
        keyword(ENTITY_TYPE_FIELD, "entity subtype filtering"),
        keyword(THREAD_STATUS_FIELD, "thread lifecycle filtering"),
        keyword(SCHEMA_VERSION_FIELD, "payload/schema migration filtering"),
        keyword(RETENTION_STATE_FIELD, "retention lifecycle filtering"),
        keyword(EPISODE_IDS_FIELD, "episode relationship hint filtering"),
        keyword(
            OBSERVATION_IDS_FIELD,
            "observation relationship hint filtering",
        ),
        keyword(THREAD_IDS_FIELD, "thread relationship hint filtering"),
        keyword(ENTITY_IDS_FIELD, "entity relationship hint filtering"),
        keyword(
            PARTICIPANT_ENTITY_IDS_FIELD,
            "episode participant relationship hint filtering",
        ),
        keyword(
            SPEAKER_ENTITY_ID_FIELD,
            "observation speaker relationship hint filtering",
        ),
        keyword(SUPERSEDES_FIELD, "supersession hint filtering"),
        keyword(MODALITY_FIELD, "source modality filtering"),
        keyword(
            SOURCE_CONVERSATION_ID_FIELD,
            "source conversation filtering",
        ),
        keyword(CANONICAL_KEY_FIELD, "canonical key filtering"),
        datetime(CREATED_AT_FIELD, "creation time filtering"),
        datetime(UPDATED_AT_FIELD, "update time filtering"),
        datetime(STARTED_AT_FIELD, "episode start time filtering"),
        datetime(ENDED_AT_FIELD, "episode end time filtering"),
        datetime(OBSERVED_AT_FIELD, "observation time filtering"),
        datetime(LAST_TOUCHED_AT_FIELD, "thread recency filtering"),
        boolean(IS_CURRENT_FIELD, "currentness filtering"),
        boolean(IS_SUPERSEDED_FIELD, "supersession/currentness filtering"),
        float(SALIENCE_SCORE_FIELD, "salience threshold filtering"),
        float(CONFIDENCE_FIELD, "confidence threshold filtering"),
        keyword(STABILITY_FIELD, "derived memory stability filtering"),
        keyword(
            RAW_REF_FIELD,
            "source pointer lookup without raw transcript storage",
        ),
    ]
}

#[derive(Debug, Serialize)]
struct QdrantVectorPayload {
    object_id: String,
    graph_uri: String,
    object_type: String,
    record_type: String,
    schema_version: String,
    surface: &'static str,
    embedding_text: String,
    content_text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    derived_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retention_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_current: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_superseded: Option<bool>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    episode_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    observation_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    thread_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    entity_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    participant_entity_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_entity_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supersedes: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    modality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonical_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ended_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_touched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    salience_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_ref: Option<String>,
}

impl From<&VectorRecord> for QdrantVectorPayload {
    fn from(record: &VectorRecord) -> Self {
        let hints = &record.payload_hints;
        let relationships = &record.relationship_hints;
        Self {
            object_id: record.object_id.to_string(),
            graph_uri: record.graph_uri.clone(),
            object_type: enum_value(record.object_type),
            record_type: enum_value(record.object_type),
            schema_version: record.schema_version.clone(),
            surface: vector_surface(record.surface),
            embedding_text: record.embedding_text.clone(),
            content_text: record.content_text.clone(),
            derived_type: hints.derived_type.map(enum_value),
            entity_type: hints.entity_type.map(enum_value),
            thread_status: hints.thread_status.map(enum_value),
            retention_state: record.retention_state.map(enum_value),
            is_current: record.is_current,
            is_superseded: hints
                .is_superseded
                .or_else(|| record.is_current.map(|value| !value)),
            episode_ids: ids(&relationships.episode_ids),
            observation_ids: ids(&relationships.observation_ids),
            thread_ids: ids(&relationships.thread_ids),
            entity_ids: ids(&relationships.entity_ids),
            participant_entity_ids: ids(&relationships.participant_entity_ids),
            speaker_entity_id: relationships.speaker_entity_id.map(|id| id.to_string()),
            supersedes: ids(&relationships.supersedes),
            modality: hints.modality.map(enum_value),
            source_conversation_id: hints.source_conversation_id.clone(),
            canonical_key: hints.canonical_key.clone(),
            created_at: hints.created_at.map(timestamp),
            updated_at: hints.updated_at.map(timestamp),
            started_at: hints.started_at.map(timestamp),
            ended_at: hints.ended_at.map(timestamp),
            observed_at: hints.observed_at.map(timestamp),
            last_touched_at: hints.last_touched_at.map(timestamp),
            salience_score: hints.salience_score,
            confidence: hints.confidence,
            stability: hints.stability.map(enum_value),
            raw_ref: record.raw_ref.clone(),
        }
    }
}

fn keyword(name: &'static str, purpose: &'static str) -> QdrantPayloadIndexField {
    QdrantPayloadIndexField {
        name,
        field_type: FieldType::Keyword,
        purpose,
    }
}

fn datetime(name: &'static str, purpose: &'static str) -> QdrantPayloadIndexField {
    QdrantPayloadIndexField {
        name,
        field_type: FieldType::Datetime,
        purpose,
    }
}

fn boolean(name: &'static str, purpose: &'static str) -> QdrantPayloadIndexField {
    QdrantPayloadIndexField {
        name,
        field_type: FieldType::Bool,
        purpose,
    }
}

fn float(name: &'static str, purpose: &'static str) -> QdrantPayloadIndexField {
    QdrantPayloadIndexField {
        name,
        field_type: FieldType::Float,
        purpose,
    }
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
    use crate::api::types::{
        graph_uri, DerivedType, ObjectType, RetentionState, Stability, DEFAULT_SCHEMA_VERSION,
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
        assert_eq!(payload[RECORD_TYPE_FIELD], json!("derived_memory"));
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
    fn index_helper_covers_high_value_filter_fields() {
        let fields = qdrant_payload_index_fields();
        let names: Vec<_> = fields.iter().map(|field| field.name).collect();

        for expected in [
            OBJECT_TYPE_FIELD,
            RECORD_TYPE_FIELD,
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
            assert!(names.contains(&expected), "missing index field {expected}");
        }

        assert_eq!(field_type(&fields, CREATED_AT_FIELD), FieldType::Datetime);
        assert_eq!(field_type(&fields, IS_CURRENT_FIELD), FieldType::Bool);
        assert_eq!(field_type(&fields, SALIENCE_SCORE_FIELD), FieldType::Float);
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

    fn field_type(fields: &[QdrantPayloadIndexField], name: &str) -> FieldType {
        fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.field_type)
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
