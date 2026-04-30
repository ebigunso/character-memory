// v0.1 Qdrant candidate-store adapter. Qdrant provides vector recall and
// payload prefiltering; Oxigraph remains authoritative for graph/lifecycle
// truth.
#![allow(dead_code)]

use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use qdrant_client::qdrant::{
    points_selector::PointsSelectorOneOf, value::Kind, vectors_config, Condition,
    CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, DatetimeRange, DeletePointsBuilder,
    Distance, Filter, PointStruct, ScoredPoint, SearchPointsBuilder, Timestamp,
    UpsertPointsBuilder, VectorParams, VectorsConfig,
};
use qdrant_client::{config::QdrantConfig, Qdrant};

use crate::api::types::{MemoryId, ObjectType};
use crate::errors::CustomError;
use crate::internal::models::vector::{
    VectorCandidateFilters, VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
    VectorSurface, VectorTimeField, VectorTimeRangeFilter,
};
use crate::internal::repositories::VectorCandidateStore;

use super::qdrant_payload::{
    qdrant_payload_index_fields, qdrant_payload_map, CREATED_AT_FIELD, ENDED_AT_FIELD,
    ENTITY_IDS_FIELD, EPISODE_IDS_FIELD, IS_CURRENT_FIELD, IS_SUPERSEDED_FIELD,
    LAST_TOUCHED_AT_FIELD, OBJECT_ID_FIELD, OBJECT_TYPE_FIELD, OBSERVED_AT_FIELD,
    PARTICIPANT_ENTITY_IDS_FIELD, RETENTION_STATE_FIELD, SPEAKER_ENTITY_ID_FIELD, STARTED_AT_FIELD,
    SURFACE_FIELD, THREAD_IDS_FIELD, UPDATED_AT_FIELD,
};

const QDRANT_CANDIDATE_TIMEOUT_SECS: u64 = 30;

pub(crate) struct QdrantVectorCandidateStore {
    client: Qdrant,
    collection_name: String,
    vector_size: u64,
}

impl QdrantVectorCandidateStore {
    pub(crate) fn new(
        url: impl AsRef<str>,
        collection_name: impl Into<String>,
        vector_size: u64,
    ) -> Result<Self, CustomError> {
        let client = Qdrant::new(qdrant_candidate_config(url.as_ref()))?;
        Ok(Self {
            client,
            collection_name: collection_name.into(),
            vector_size,
        })
    }

    pub(crate) async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await?;
        if !collections
            .collections
            .iter()
            .any(|collection| collection.name == self.collection_name)
        {
            let vectors_config = VectorsConfig {
                config: Some(vectors_config::Config::Params(VectorParams {
                    size: self.vector_size,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
            };

            let create_req = CreateCollectionBuilder::new(&self.collection_name)
                .vectors_config(vectors_config)
                .build();
            self.client.create_collection(create_req).await?;
        }

        self.ensure_payload_indexes().await
    }

    pub(crate) async fn ensure_payload_indexes(&self) -> Result<(), CustomError> {
        let info = self.client.collection_info(&self.collection_name).await?;
        let collection_info = info.result.as_ref().ok_or_else(|| {
            CustomError::DatabaseError(format!(
                "Qdrant collection info response was missing result for collection '{}'",
                self.collection_name
            ))
        })?;
        validate_collection_vector_config(
            &self.collection_name,
            self.vector_size,
            collection_info
                .config
                .as_ref()
                .and_then(|config| config.params.as_ref())
                .and_then(|params| params.vectors_config.as_ref()),
        )?;

        let empty_payload_schema: HashMap<String, qdrant_client::qdrant::PayloadSchemaInfo> =
            HashMap::new();
        let payload_schema = if collection_info.payload_schema.is_empty() {
            &empty_payload_schema
        } else {
            &collection_info.payload_schema
        };

        for field in qdrant_payload_index_fields() {
            if payload_schema.contains_key(field.name) {
                continue;
            }

            self.client
                .create_field_index(CreateFieldIndexCollectionBuilder::new(
                    &self.collection_name,
                    field.name,
                    field.field_type,
                ))
                .await?;
        }

        Ok(())
    }

    async fn upsert_points(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        let points = qdrant_point_structs(records)?;
        let request = UpsertPointsBuilder::new(&self.collection_name, points)
            .wait(true)
            .timeout(QDRANT_CANDIDATE_TIMEOUT_SECS)
            .build();
        self.client.upsert_points(request).await?;
        Ok(())
    }
}

fn validate_collection_vector_config(
    collection_name: &str,
    expected_vector_size: u64,
    vectors_config: Option<&VectorsConfig>,
) -> Result<(), CustomError> {
    let Some(vectors_config) = vectors_config else {
        return Err(CustomError::DatabaseError(format!(
            "Qdrant collection '{collection_name}' is missing vector configuration; expected unnamed vectors with size {expected_vector_size}."
        )));
    };

    match vectors_config.config.as_ref() {
        Some(vectors_config::Config::Params(params))
            if params.size == expected_vector_size
                && params.distance == Distance::Cosine as i32 =>
        {
            Ok(())
        }
        Some(vectors_config::Config::Params(params))
            if params.size == expected_vector_size =>
        {
            Err(CustomError::DatabaseError(format!(
                "Qdrant collection '{collection_name}' vector distance mismatch: expected Cosine, found {}.",
                Distance::try_from(params.distance)
                    .map(|distance| distance.as_str_name().to_owned())
                    .unwrap_or_else(|_| params.distance.to_string())
            )))
        }
        Some(vectors_config::Config::Params(params)) => Err(CustomError::DatabaseError(format!(
            "Qdrant collection '{collection_name}' vector size mismatch: expected {expected_vector_size}, found {}.",
            params.size
        ))),
        Some(vectors_config::Config::ParamsMap(params_map)) => {
            let mut vector_names = params_map.map.keys().cloned().collect::<Vec<_>>();
            vector_names.sort();
            Err(CustomError::DatabaseError(format!(
                "Qdrant collection '{collection_name}' uses named vectors ({}) but CharacterMemory expects an unnamed vector with size {expected_vector_size}.",
                vector_names.join(", ")
            )))
        }
        None => Err(CustomError::DatabaseError(format!(
            "Qdrant collection '{collection_name}' vector configuration is empty; expected unnamed vectors with size {expected_vector_size}."
        ))),
    }
}

#[async_trait]
impl VectorCandidateStore for QdrantVectorCandidateStore {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        self.upsert_points(records).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        let mut builder = SearchPointsBuilder::new(
            &self.collection_name,
            query.query_embedding.clone(),
            query.limit as u64,
        )
        .with_payload(true)
        .with_vectors(false);

        if let Some(filter) = qdrant_candidate_filter(query) {
            builder = builder.filter(filter);
        }

        let response = self.client.search_points(builder.build()).await?;
        response
            .result
            .into_iter()
            .map(scored_point_to_match)
            .collect()
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        if object_ids.is_empty() {
            return Ok(());
        }

        let conditions: Vec<_> = object_ids
            .iter()
            .map(|id| Condition::matches(OBJECT_ID_FIELD, id.to_string()))
            .collect();
        let selector = PointsSelectorOneOf::Filter(Filter::should(conditions));
        let request = DeletePointsBuilder::new(&self.collection_name)
            .points(selector)
            .wait(true)
            .timeout(QDRANT_CANDIDATE_TIMEOUT_SECS)
            .build();
        self.client.delete_points(request).await?;
        Ok(())
    }
}

fn qdrant_candidate_config(url: &str) -> QdrantConfig {
    QdrantConfig::from_url(url).timeout(Duration::from_secs(QDRANT_CANDIDATE_TIMEOUT_SECS))
}

fn qdrant_candidate_filter(query: &VectorCandidateSearch) -> Option<Filter> {
    let mut must_conditions = Vec::new();

    if !query.object_types.is_empty() {
        must_conditions.push(any_field_matches(
            OBJECT_TYPE_FIELD,
            query
                .object_types
                .iter()
                .map(|value| object_type_name(*value)),
        ));
    }

    must_conditions.extend(qdrant_filter_conditions(&query.filters));
    if let Some(condition) = currentness_filter_condition(query) {
        must_conditions.push(condition);
    }

    if must_conditions.is_empty() {
        None
    } else {
        Some(Filter::must(must_conditions))
    }
}

fn qdrant_filter_conditions(filters: &VectorCandidateFilters) -> Vec<Condition> {
    let mut conditions = Vec::new();

    if !filters.retention_states.is_empty() {
        conditions.push(any_field_matches(
            RETENTION_STATE_FIELD,
            filters
                .retention_states
                .iter()
                .map(|value| retention_state_name(*value)),
        ));
    }

    if !filters.thread_ids.is_empty() {
        conditions.push(any_field_matches(
            THREAD_IDS_FIELD,
            filters.thread_ids.iter().map(ToString::to_string),
        ));
    }

    if !filters.episode_ids.is_empty() {
        conditions.push(any_field_matches(
            EPISODE_IDS_FIELD,
            filters.episode_ids.iter().map(ToString::to_string),
        ));
    }

    if !filters.entity_ids.is_empty() {
        let mut entity_conditions = Vec::new();
        for field in [
            ENTITY_IDS_FIELD,
            PARTICIPANT_ENTITY_IDS_FIELD,
            SPEAKER_ENTITY_ID_FIELD,
        ] {
            entity_conditions.push(any_field_matches(
                field,
                filters.entity_ids.iter().map(ToString::to_string),
            ));
        }
        conditions.push(Condition::from(Filter::min_should(1, entity_conditions)));
    }

    conditions.extend(filters.time_ranges.iter().map(|time_range| {
        Condition::datetime_range(
            time_field_name(time_range.field),
            datetime_range(time_range),
        )
    }));

    conditions
}

fn currentness_filter_condition(query: &VectorCandidateSearch) -> Option<Condition> {
    if !query.filters.has_currentness_filters() {
        return None;
    }

    let currentness_conditions = currentness_conditions(&query.filters);
    if currentness_conditions.is_empty() {
        return None;
    }

    let mut branches = Vec::new();
    if query.object_types.is_empty() {
        branches.push(Condition::from(Filter {
            must: Vec::new(),
            should: Vec::new(),
            must_not: vec![Condition::matches(
                OBJECT_TYPE_FIELD,
                object_type_name(ObjectType::DerivedMemory).to_owned(),
            )],
            min_should: None,
        }));
    } else {
        let non_derived_types = query
            .object_types
            .iter()
            .copied()
            .filter(|object_type| *object_type != ObjectType::DerivedMemory)
            .collect::<Vec<_>>();
        if !non_derived_types.is_empty() {
            branches.push(any_field_matches(
                OBJECT_TYPE_FIELD,
                non_derived_types.into_iter().map(object_type_name),
            ));
        }
    }

    if query.object_types.is_empty() || query.object_types.contains(&ObjectType::DerivedMemory) {
        let mut derived_conditions = vec![Condition::matches(
            OBJECT_TYPE_FIELD,
            object_type_name(ObjectType::DerivedMemory).to_owned(),
        )];
        derived_conditions.extend(currentness_conditions);
        branches.push(Condition::from(Filter::must(derived_conditions)));
    }

    match branches.len() {
        0 => None,
        1 => branches.into_iter().next(),
        _ => Some(Condition::from(Filter::min_should(1, branches))),
    }
}

fn currentness_conditions(filters: &VectorCandidateFilters) -> Vec<Condition> {
    let mut conditions = Vec::new();
    if let Some(is_current) = filters.is_current {
        conditions.push(payload_hint_matches_or_missing(
            IS_CURRENT_FIELD,
            is_current,
        ));
    }
    if let Some(is_superseded) = filters.is_superseded {
        conditions.push(payload_hint_matches_or_missing(
            IS_SUPERSEDED_FIELD,
            is_superseded,
        ));
    }
    conditions
}

fn payload_hint_matches_or_missing(field: &str, value: bool) -> Condition {
    Condition::from(Filter::min_should(
        1,
        vec![
            Condition::matches(field, value),
            Condition::is_empty(field),
            Condition::is_null(field),
        ],
    ))
}

fn any_field_matches(
    field: &str,
    values: impl IntoIterator<Item = impl Into<String>>,
) -> Condition {
    let conditions = values
        .into_iter()
        .map(|value| Condition::matches(field, value.into()))
        .collect::<Vec<_>>();

    if conditions.len() == 1 {
        conditions.into_iter().next().unwrap()
    } else {
        Condition::from(Filter::min_should(1, conditions))
    }
}

fn datetime_range(time_range: &VectorTimeRangeFilter) -> DatetimeRange {
    DatetimeRange {
        gte: time_range.after.map(timestamp),
        lte: time_range.before.map(timestamp),
        ..DatetimeRange::default()
    }
}

fn timestamp(value: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

fn qdrant_point_structs(
    records: &[VectorRecordEmbedding<'_>],
) -> Result<Vec<PointStruct>, CustomError> {
    records
        .iter()
        .map(|record| {
            let payload = qdrant_payload_map(record.record)?;
            Ok(PointStruct::new(
                qdrant_point_id(record.record).to_string(),
                record.embedding.to_vec(),
                payload,
            ))
        })
        .collect()
}

fn scored_point_to_match(point: ScoredPoint) -> Result<VectorCandidateMatch, CustomError> {
    let object_id = payload_string(&point.payload, OBJECT_ID_FIELD)?;
    let object_id = uuid::Uuid::parse_str(&object_id).map_err(|error| {
        CustomError::DatabaseError(format!("Invalid Qdrant object_id payload UUID: {error}"))
    })?;

    let object_type = parse_object_type(payload_string(&point.payload, OBJECT_TYPE_FIELD)?)?;
    let surface = parse_vector_surface(payload_string(&point.payload, SURFACE_FIELD)?)?;

    Ok(VectorCandidateMatch::new(
        object_id,
        object_type,
        surface,
        point.score,
    ))
}

fn qdrant_point_id(record: &crate::internal::models::vector::VectorRecord) -> uuid::Uuid {
    let mut first = 0xcbf29ce484222325_u64;
    let mut second = 0x9e3779b97f4a7c15_u64;

    for byte in record
        .object_id
        .as_bytes()
        .iter()
        .copied()
        .chain(surface_name(record.surface).as_bytes().iter().copied())
    {
        first ^= u64::from(byte);
        first = first.wrapping_mul(0x100000001b3);
        second ^= u64::from(byte).wrapping_add(0x9e3779b97f4a7c15);
        second = second.rotate_left(5).wrapping_mul(0x517cc1b727220a95);
    }

    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&first.to_be_bytes());
    bytes[8..].copy_from_slice(&second.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn payload_string(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    field: &str,
) -> Result<String, CustomError> {
    match payload.get(field).and_then(|value| value.kind.as_ref()) {
        Some(Kind::StringValue(value)) => Ok(value.clone()),
        _ => Err(CustomError::DatabaseError(format!(
            "Missing or invalid string field in Qdrant payload: {field}"
        ))),
    }
}

fn object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

fn retention_state_name(retention_state: crate::api::types::RetentionState) -> &'static str {
    match retention_state {
        crate::api::types::RetentionState::Active => "active",
        crate::api::types::RetentionState::Suppressed => "suppressed",
        crate::api::types::RetentionState::Archived => "archived",
        crate::api::types::RetentionState::Deleted => "deleted",
    }
}

fn time_field_name(field: VectorTimeField) -> &'static str {
    match field {
        VectorTimeField::Created => CREATED_AT_FIELD,
        VectorTimeField::Updated => UPDATED_AT_FIELD,
        VectorTimeField::Started => STARTED_AT_FIELD,
        VectorTimeField::Ended => ENDED_AT_FIELD,
        VectorTimeField::Observed => OBSERVED_AT_FIELD,
        VectorTimeField::LastTouched => LAST_TOUCHED_AT_FIELD,
    }
}

fn surface_name(surface: VectorSurface) -> &'static str {
    match surface {
        VectorSurface::Summary => "summary",
        VectorSurface::Text => "text",
        VectorSurface::Name => "name",
        VectorSurface::DerivedText => "derived_text",
        VectorSurface::Query => "query",
    }
}

fn parse_object_type(value: String) -> Result<ObjectType, CustomError> {
    match value.as_str() {
        "episode" => Ok(ObjectType::Episode),
        "observation" => Ok(ObjectType::Observation),
        "entity" => Ok(ObjectType::Entity),
        "memory_thread" => Ok(ObjectType::MemoryThread),
        "derived_memory" => Ok(ObjectType::DerivedMemory),
        "memory_link" => Ok(ObjectType::MemoryLink),
        _ => Err(CustomError::DatabaseError(format!(
            "Unknown Qdrant object_type payload value: {value}"
        ))),
    }
}

fn parse_vector_surface(value: String) -> Result<VectorSurface, CustomError> {
    match value.as_str() {
        "summary" => Ok(VectorSurface::Summary),
        "text" => Ok(VectorSurface::Text),
        "name" => Ok(VectorSurface::Name),
        "derived_text" => Ok(VectorSurface::DerivedText),
        "query" => Ok(VectorSurface::Query),
        _ => Err(CustomError::DatabaseError(format!(
            "Unknown Qdrant surface payload value: {value}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::super::qdrant_payload::{CONTENT_TEXT_FIELD, GRAPH_URI_FIELD};
    use super::*;
    use crate::api::types::{graph_uri, RetentionState, DEFAULT_SCHEMA_VERSION};
    use crate::internal::models::vector::{
        VectorCandidateFilters, VectorPayloadHints, VectorRecord, VectorRecordEmbedding,
        VectorRelationshipHints, VectorSurface, VectorTimeField, VectorTimeRangeFilter,
    };
    use qdrant_client::qdrant::condition::ConditionOneOf;
    use qdrant_client::qdrant::{
        point_id::PointIdOptions, value::Kind, vector, vectors, PointId, Value, VectorParamsMap,
    };
    use std::env;
    use uuid::Uuid;

    #[test]
    fn candidate_client_config_extends_default_request_timeout() {
        let config = qdrant_candidate_config("http://localhost:6334");

        assert_eq!(
            config.timeout,
            Duration::from_secs(QDRANT_CANDIDATE_TIMEOUT_SECS)
        );
        assert_eq!(config.uri, "http://localhost:6334");
    }

    #[test]
    fn validates_existing_collection_vector_size() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::Params(VectorParams {
                size: 1536,
                distance: Distance::Cosine.into(),
                ..Default::default()
            })),
        };

        assert!(validate_collection_vector_config("memories", 1536, Some(&config)).is_ok());

        let error = validate_collection_vector_config("memories", 3072, Some(&config))
            .expect_err("mismatched existing collection should fail");
        assert!(matches!(
            error,
            CustomError::DatabaseError(message)
                if message.contains("memories")
                    && message.contains("expected 3072")
                    && message.contains("found 1536")
        ));
    }

    #[test]
    fn rejects_existing_collection_with_wrong_distance_metric() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::Params(VectorParams {
                size: 1536,
                distance: Distance::Euclid.into(),
                ..Default::default()
            })),
        };

        let error = validate_collection_vector_config("memories", 1536, Some(&config))
            .expect_err("same-size collection with wrong distance should fail");
        assert!(matches!(
            error,
            CustomError::DatabaseError(message)
                if message.contains("memories")
                    && message.contains("expected Cosine")
                    && message.contains("Euclid")
        ));
    }

    #[test]
    fn rejects_named_vector_collection_config() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::ParamsMap(VectorParamsMap {
                map: HashMap::from([(
                    "content".to_owned(),
                    VectorParams {
                        size: 1536,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    },
                )]),
            })),
        };

        let error = validate_collection_vector_config("memories", 1536, Some(&config))
            .expect_err("named vectors should not be accepted for unnamed vector store");
        assert!(matches!(
            error,
            CustomError::DatabaseError(message)
                if message.contains("named vectors") && message.contains("content")
        ));
    }

    #[test]
    fn search_result_mapping_reads_payload_identity_and_surface() {
        let object_id = Uuid::new_v4();
        let point_id = Uuid::new_v4();
        let point = ScoredPoint {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(point_id.to_string())),
            }),
            payload: HashMap::from([
                (
                    OBJECT_ID_FIELD.to_owned(),
                    string_value(&object_id.to_string()),
                ),
                (OBJECT_TYPE_FIELD.to_owned(), string_value("derived_memory")),
                (SURFACE_FIELD.to_owned(), string_value("derived_text")),
            ]),
            score: 0.75,
            ..Default::default()
        };

        let matched = scored_point_to_match(point).expect("point maps");

        assert_eq!(matched.object_id, object_id);
        assert_eq!(matched.object_type, ObjectType::DerivedMemory);
        assert_eq!(matched.surface, VectorSurface::DerivedText);
        assert_eq!(matched.score, 0.75);
    }

    #[test]
    fn point_ids_are_unique_per_object_surface_and_identity_stays_in_payload() {
        let object_id = Uuid::new_v4();
        let summary = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            graph_uri(ObjectType::Episode, object_id),
            VectorSurface::Summary,
            "Episode summary.",
            "Episode summary.",
            DEFAULT_SCHEMA_VERSION,
            None,
            None,
            VectorRelationshipHints::default(),
            None,
        );
        let text = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            graph_uri(ObjectType::Episode, object_id),
            VectorSurface::Text,
            "Episode text.",
            "Episode text.",
            DEFAULT_SCHEMA_VERSION,
            None,
            None,
            VectorRelationshipHints::default(),
            None,
        );

        let points = qdrant_point_structs(&[
            VectorRecordEmbedding::new(&summary, &[1.0, 0.0]),
            VectorRecordEmbedding::new(&text, &[0.0, 1.0]),
        ])
        .expect("points build");

        assert_ne!(points[0].id, points[1].id);
        assert_eq!(
            payload_string(&points[0].payload, OBJECT_ID_FIELD).unwrap(),
            object_id.to_string()
        );
        assert_eq!(
            payload_string(&points[1].payload, OBJECT_ID_FIELD).unwrap(),
            object_id.to_string()
        );
    }

    #[test]
    fn upsert_points_use_full_vector_record_payloads() {
        let object_id = Uuid::new_v4();
        let related_episode_id = Uuid::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Qdrant keeps payload details.",
            "Qdrant keeps payload details.",
            DEFAULT_SCHEMA_VERSION,
            None,
            Some(true),
            VectorRelationshipHints {
                episode_ids: vec![related_episode_id],
                ..VectorRelationshipHints::default()
            },
            Some("raw://conversation/chat_123#turn_42".to_owned()),
        );

        let points = qdrant_point_structs(&[VectorRecordEmbedding::new(&record, &[0.25, 0.75])])
            .expect("points build");

        assert_eq!(points.len(), 1);
        assert_eq!(
            payload_string(&points[0].payload, OBJECT_TYPE_FIELD).unwrap(),
            "derived_memory"
        );
        assert_eq!(
            payload_string(&points[0].payload, SURFACE_FIELD).unwrap(),
            "derived_text"
        );
        assert_eq!(
            payload_string(&points[0].payload, GRAPH_URI_FIELD).unwrap(),
            record.graph_uri
        );
        assert_eq!(
            payload_string(&points[0].payload, CONTENT_TEXT_FIELD).unwrap(),
            "Qdrant keeps payload details."
        );
        assert!(points[0].payload.contains_key("episode_ids"));
        assert!(points[0].payload.contains_key("raw_ref"));

        let vector = points[0]
            .vectors
            .as_ref()
            .and_then(|vectors| vectors.vectors_options.as_ref())
            .expect("vectors present");
        match vector {
            vectors::VectorsOptions::Vector(vector) => match vector.vector.as_ref() {
                Some(vector::Vector::Dense(dense)) => assert_eq!(dense.data, vec![0.25, 0.75]),
                _ => panic!("expected dense vector"),
            },
            _ => panic!("expected unnamed vector"),
        }
    }

    #[test]
    fn candidate_prefilter_construction_maps_payload_hint_fields() {
        let thread_id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let episode_id = Uuid::new_v4();
        let filters = VectorCandidateFilters::new()
            .with_retention_states(vec![RetentionState::Active])
            .current_only()
            .with_thread_ids(vec![thread_id])
            .with_entity_ids(vec![entity_id])
            .with_episode_ids(vec![episode_id])
            .with_time_range(VectorTimeRangeFilter::new(
                VectorTimeField::Updated,
                Some(timestamp_utc("2026-04-29T10:00:00Z")),
                Some(timestamp_utc("2026-04-29T11:00:00Z")),
            ));
        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_object_types(vec![ObjectType::DerivedMemory, ObjectType::Observation])
            .with_filters(filters);

        let filter = qdrant_candidate_filter(&query).expect("filter builds");
        let keys = field_keys(&filter);

        for expected in [
            OBJECT_TYPE_FIELD,
            RETENTION_STATE_FIELD,
            THREAD_IDS_FIELD,
            EPISODE_IDS_FIELD,
            ENTITY_IDS_FIELD,
            PARTICIPANT_ENTITY_IDS_FIELD,
            SPEAKER_ENTITY_ID_FIELD,
            UPDATED_AT_FIELD,
        ] {
            assert!(keys.contains(&expected.to_owned()), "missing {expected}");
        }
        assert!(keys.contains(&IS_CURRENT_FIELD.to_owned()));
        assert!(keys.contains(&IS_SUPERSEDED_FIELD.to_owned()));
    }

    #[test]
    fn candidate_prefilter_scopes_currentness_to_derived_memory_searches() {
        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_object_types(vec![ObjectType::DerivedMemory])
            .with_filters(VectorCandidateFilters::new().current_only());

        let filter = qdrant_candidate_filter(&query).expect("filter builds");
        let keys = field_keys(&filter);

        assert!(keys.contains(&IS_CURRENT_FIELD.to_owned()));
        assert!(keys.contains(&IS_SUPERSEDED_FIELD.to_owned()));
    }

    #[test]
    fn candidate_prefilter_allows_missing_currentness_hints_for_graph_verification() {
        let condition = currentness_filter_condition(
            &VectorCandidateSearch::new(vec![1.0, 0.0], 10)
                .with_object_types(vec![ObjectType::DerivedMemory])
                .with_filters(VectorCandidateFilters::new().current_only()),
        )
        .expect("currentness filter builds");
        let mut condition_kinds = Vec::new();
        collect_condition_kinds(&condition, &mut condition_kinds);

        assert!(condition_kinds.contains(&"field:is_current".to_owned()));
        assert!(condition_kinds.contains(&"is_empty:is_current".to_owned()));
        assert!(condition_kinds.contains(&"is_null:is_current".to_owned()));
        assert!(condition_kinds.contains(&"field:is_superseded".to_owned()));
        assert!(condition_kinds.contains(&"is_empty:is_superseded".to_owned()));
        assert!(condition_kinds.contains(&"is_null:is_superseded".to_owned()));
    }

    #[test]
    fn candidate_mapping_preserves_qdrant_result_order_and_scores() {
        let higher_score_id = Uuid::new_v4();
        let lower_score_id = Uuid::new_v4();
        let points = vec![
            scored_point(higher_score_id, ObjectType::DerivedMemory, 0.91),
            scored_point(lower_score_id, ObjectType::Observation, 0.42),
        ];

        let matches = points
            .into_iter()
            .map(scored_point_to_match)
            .collect::<Result<Vec<_>, _>>()
            .expect("points map");

        assert_eq!(matches[0].object_id, higher_score_id);
        assert_eq!(matches[0].score, 0.91);
        assert_eq!(matches[1].object_id, lower_score_id);
        assert_eq!(matches[1].score, 0.42);
    }

    #[test]
    fn candidate_mapping_does_not_return_lifecycle_hints_as_authority() {
        let object_id = Uuid::new_v4();
        let mut point = scored_point(object_id, ObjectType::DerivedMemory, 0.77);
        point.payload.insert(
            RETENTION_STATE_FIELD.to_owned(),
            string_value(retention_state_name(RetentionState::Active)),
        );
        point
            .payload
            .insert(IS_CURRENT_FIELD.to_owned(), bool_value(true));

        let matched = scored_point_to_match(point).expect("point maps");

        assert_eq!(matched.object_id, object_id);
        assert_eq!(matched.object_type, ObjectType::DerivedMemory);
        assert_eq!(matched.surface, VectorSurface::DerivedText);
    }

    #[tokio::test]
    #[ignore = "requires local Qdrant: docker compose -f docker-compose.qdrant.yml up -d and QDRANT_CONNECTION_STRING"]
    async fn qdrant_candidate_store_live_smoke_upserts_filters_searches_and_deletes() {
        let url = env::var("QDRANT_CONNECTION_STRING")
            .expect("QDRANT_CONNECTION_STRING is required for live Qdrant smoke test");
        let collection_name = format!("cmem_candidate_smoke_{}", Uuid::new_v4());
        let store =
            QdrantVectorCandidateStore::new(url, &collection_name, 2).expect("store builds");

        let object_id = Uuid::new_v4();
        let thread_id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let episode_id = Uuid::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Qdrant keeps filter hints.",
            "Qdrant keeps filter hints.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            Some(true),
            VectorRelationshipHints {
                episode_ids: vec![episode_id],
                thread_ids: vec![thread_id],
                entity_ids: vec![entity_id],
                ..VectorRelationshipHints::default()
            },
            None,
        )
        .with_payload_hints(VectorPayloadHints {
            updated_at: Some(timestamp_utc("2026-04-29T10:30:00Z")),
            is_superseded: Some(false),
            ..VectorPayloadHints::default()
        });

        store.init_collection().await.expect("collection init");
        store
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
            .await
            .expect("upsert succeeds");

        let matches = store
            .search_candidates(
                &VectorCandidateSearch::new(vec![1.0, 0.0], 1)
                    .with_object_types(vec![ObjectType::DerivedMemory])
                    .with_filters(
                        VectorCandidateFilters::new()
                            .with_retention_states(vec![RetentionState::Active])
                            .current_only()
                            .with_thread_ids(vec![thread_id])
                            .with_entity_ids(vec![entity_id])
                            .with_episode_ids(vec![episode_id])
                            .with_time_range(VectorTimeRangeFilter::new(
                                VectorTimeField::Updated,
                                Some(timestamp_utc("2026-04-29T10:00:00Z")),
                                Some(timestamp_utc("2026-04-29T11:00:00Z")),
                            )),
                    ),
            )
            .await
            .expect("search succeeds");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].object_id, object_id);

        store
            .delete_candidates(&[object_id])
            .await
            .expect("delete succeeds");
        let _ = store.client.delete_collection(&collection_name).await;
    }

    fn field_keys(filter: &Filter) -> Vec<String> {
        let mut keys = Vec::new();
        for condition in filter
            .must
            .iter()
            .chain(filter.should.iter())
            .chain(filter.must_not.iter())
        {
            collect_condition_field_keys(condition, &mut keys);
        }
        if let Some(min_should) = &filter.min_should {
            for condition in &min_should.conditions {
                collect_condition_field_keys(condition, &mut keys);
            }
        }
        keys
    }

    fn collect_condition_field_keys(condition: &Condition, keys: &mut Vec<String>) {
        match &condition.condition_one_of {
            Some(ConditionOneOf::Field(field)) => keys.push(field.key.clone()),
            Some(ConditionOneOf::Filter(filter)) => keys.extend(field_keys(filter)),
            _ => {}
        }
    }

    fn collect_condition_kinds(condition: &Condition, kinds: &mut Vec<String>) {
        match &condition.condition_one_of {
            Some(ConditionOneOf::Field(field)) => kinds.push(format!("field:{}", field.key)),
            Some(ConditionOneOf::IsEmpty(field)) => kinds.push(format!("is_empty:{}", field.key)),
            Some(ConditionOneOf::IsNull(field)) => kinds.push(format!("is_null:{}", field.key)),
            Some(ConditionOneOf::Filter(filter)) => {
                for condition in filter
                    .must
                    .iter()
                    .chain(filter.should.iter())
                    .chain(filter.must_not.iter())
                {
                    collect_condition_kinds(condition, kinds);
                }
                if let Some(min_should) = &filter.min_should {
                    for condition in &min_should.conditions {
                        collect_condition_kinds(condition, kinds);
                    }
                }
            }
            _ => {}
        }
    }

    fn scored_point(object_id: Uuid, object_type: ObjectType, score: f32) -> ScoredPoint {
        ScoredPoint {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(Uuid::new_v4().to_string())),
            }),
            payload: HashMap::from([
                (
                    OBJECT_ID_FIELD.to_owned(),
                    string_value(&object_id.to_string()),
                ),
                (
                    OBJECT_TYPE_FIELD.to_owned(),
                    string_value(object_type_name(object_type)),
                ),
                (SURFACE_FIELD.to_owned(), string_value("derived_text")),
            ]),
            score,
            ..Default::default()
        }
    }

    fn timestamp_utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn string_value(value: &str) -> Value {
        Value {
            kind: Some(Kind::StringValue(value.to_owned())),
        }
    }

    fn bool_value(value: bool) -> Value {
        Value {
            kind: Some(Kind::BoolValue(value)),
        }
    }
}
