// Transitional v0.1 Qdrant candidate-store scaffold: downstream storage
// pipeline chunks will consume the concrete adapter after graph authority lands.
// Remove once remember/link production wiring or tests consume the adapter, or
// prune any remaining unused surface then.
#![allow(dead_code)]

use std::collections::HashMap;

use async_trait::async_trait;
use qdrant_client::qdrant::{
    point_id::PointIdOptions, points_selector::PointsSelectorOneOf, value::Kind, vectors_config,
    Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, DeletePointsBuilder,
    Distance, Filter, PointId, PointStruct, PointsIdsList, ScoredPoint, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParams, VectorsConfig,
};
use qdrant_client::{config::QdrantConfig, Qdrant};

use crate::api::types::{MemoryId, ObjectType};
use crate::errors::CustomError;
use crate::internal::models::vector::{
    VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding, VectorSurface,
};
use crate::internal::repositories::VectorCandidateStore;

use super::qdrant_payload::{
    qdrant_payload_index_fields, qdrant_payload_map, OBJECT_TYPE_FIELD, SURFACE_FIELD,
};

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
        let client = Qdrant::new(QdrantConfig::from_url(url.as_ref()))?;
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
        let empty_payload_schema: HashMap<String, qdrant_client::qdrant::PayloadSchemaInfo> =
            HashMap::new();
        let payload_schema = info
            .result
            .as_ref()
            .map(|result| &result.payload_schema)
            .unwrap_or(&empty_payload_schema);

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
            .build();
        self.client.upsert_points(request).await?;
        Ok(())
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

        if !query.object_types.is_empty() {
            let conditions = query
                .object_types
                .iter()
                .map(|object_type| {
                    Condition::matches(OBJECT_TYPE_FIELD, object_type_name(*object_type).to_owned())
                })
                .collect::<Vec<_>>();
            builder = builder.filter(Filter::min_should(1, conditions));
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

        let ids = object_ids
            .iter()
            .map(|id| PointId {
                point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
            })
            .collect();
        let selector = PointsSelectorOneOf::Points(PointsIdsList { ids });
        let request = DeletePointsBuilder::new(&self.collection_name)
            .points(selector)
            .wait(true)
            .build();
        self.client.delete_points(request).await?;
        Ok(())
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
                record.record.object_id.to_string(),
                record.embedding.to_vec(),
                payload,
            ))
        })
        .collect()
}

fn scored_point_to_match(point: ScoredPoint) -> Result<VectorCandidateMatch, CustomError> {
    let object_id = point
        .id
        .as_ref()
        .and_then(|id| id.point_id_options.as_ref())
        .and_then(|options| match options {
            PointIdOptions::Uuid(value) => Some(value.as_str()),
            _ => None,
        })
        .ok_or_else(|| CustomError::DatabaseError("Missing Qdrant point UUID".to_owned()))?;
    let object_id = uuid::Uuid::parse_str(object_id).map_err(|error| {
        CustomError::DatabaseError(format!("Invalid Qdrant point UUID: {error}"))
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
    use crate::api::types::{graph_uri, DEFAULT_SCHEMA_VERSION};
    use crate::internal::models::vector::{
        VectorPayloadHints, VectorRecord, VectorRecordEmbedding, VectorRelationshipHints,
        VectorSurface,
    };
    use qdrant_client::qdrant::{value::Kind, vector, vectors, Value};
    use std::env;
    use uuid::Uuid;

    #[test]
    fn search_result_mapping_reads_v0_1_payload_identity_and_surface() {
        let object_id = Uuid::new_v4();
        let point = ScoredPoint {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(object_id.to_string())),
            }),
            payload: HashMap::from([
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
    fn upsert_points_use_full_v0_1_vector_record_payloads() {
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

    #[tokio::test]
    #[ignore = "requires local Qdrant: docker compose -f docker-compose.qdrant.yml up -d and QDRANT_CONNECTION_STRING"]
    async fn qdrant_v0_1_candidate_store_live_smoke_upserts_searches_and_deletes() {
        let url = env::var("QDRANT_CONNECTION_STRING")
            .expect("QDRANT_CONNECTION_STRING is required for live Qdrant smoke test");
        let collection_name = format!("cmem_v0_1_candidate_smoke_{}", Uuid::new_v4());
        let store =
            QdrantVectorCandidateStore::new(url, &collection_name, 2).expect("store builds");

        let object_id = Uuid::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Qdrant keeps filter hints.",
            "Qdrant keeps filter hints.",
            DEFAULT_SCHEMA_VERSION,
            None,
            Some(true),
            VectorRelationshipHints::default(),
            None,
        )
        .with_payload_hints(VectorPayloadHints::default());

        store.init_collection().await.expect("collection init");
        store
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
            .await
            .expect("upsert succeeds");

        let matches = store
            .search_candidates(
                &VectorCandidateSearch::new(vec![1.0, 0.0], 1)
                    .with_object_types(vec![ObjectType::DerivedMemory]),
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

    fn string_value(value: &str) -> Value {
        Value {
            kind: Some(Kind::StringValue(value.to_owned())),
        }
    }
}
