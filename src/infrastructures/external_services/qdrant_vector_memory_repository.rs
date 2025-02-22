use async_trait::async_trait;
use chrono::{DateTime, Utc};
use qdrant_client::qdrant::{
    point_id::PointIdOptions, points_selector::PointsSelectorOneOf, CreateCollectionBuilder,
    DeletePointsBuilder, Distance, PointStruct, PointsIdsList, Range, ScoredPoint,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParams, VectorsConfig, PointId,
    Filter, Condition, vectors_config, vectors_output,
};
use qdrant_client::{Qdrant, config::QdrantConfig};
use uuid::Uuid;

use crate::config::settings::VectorMemoryRepositorySettings;
use crate::errors::CustomError;
use crate::models::internal::{MemoryEntry, VectorMetadata};
use crate::models::public::MemoryFilters;
use crate::repositories::VectorMemoryRepository;

pub struct QdrantVectorMemoryRepository {
    client: Qdrant,
    config: VectorMemoryRepositorySettings,
}

impl QdrantVectorMemoryRepository {
    pub(crate) fn new(config: VectorMemoryRepositorySettings) -> Result<Self, CustomError> {
        let client = Qdrant::new(
            QdrantConfig::from_url(&config.url)
        )?;
        Ok(Self { client, config })
    }

    // Helper: Convert a Qdrant ScoredPoint to a MemoryEntry.
    fn scored_point_to_memory_entry(&self, point: ScoredPoint) -> Result<MemoryEntry, CustomError> {
        // Extract ID
        let id = point.id.ok_or_else(|| CustomError::DatabaseError("Missing point ID".to_string()))?;
        let id_str = match id.point_id_options {
            Some(PointIdOptions::Uuid(ref s)) => s.clone(),
            _ => return Err(CustomError::DatabaseError("Invalid point id variant".to_string())),
        };
        let uuid = Uuid::parse_str(&id_str)
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {}", e)))?;

        // Extract vector
        let vectors_output = point.vectors.ok_or_else(|| CustomError::DatabaseError("Missing vector in point".to_string()))?;
        let vector = match vectors_output.vectors_options {
            Some(vectors_output::VectorsOptions::Vector(vo)) => vo.data,
            _ => return Err(CustomError::DatabaseError("Unexpected vector type".to_string())),
        };

        // Extract required fields from payload
        let memory_type = point.payload.get("memory_type")
            .ok_or_else(|| CustomError::DatabaseError("Missing memory_type in payload".to_string()))?
            .to_string()
            .trim_matches('"')
            .to_lowercase();

        let content = point.payload.get("content")
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .to_string()
            .trim_matches('"')
            .to_string();

        // Create metadata based on memory type
        let metadata = if memory_type == "semantic" {
            VectorMetadata::new_semantic(uuid, content)
        } else {
            // Extract and parse timestamp
            let timestamp_str = point.payload.get("timestamp")
                .ok_or_else(|| CustomError::MissingEpisodicField("timestamp"))?
                .to_string()
                .trim_matches('"')
                .to_string();
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| CustomError::DatabaseError(format!("Invalid timestamp format: {}", e)))?
                .with_timezone(&Utc);

            // Extract location
            let location_text = point.payload.get("location_text")
                .ok_or_else(|| CustomError::MissingEpisodicField("location_text"))?
                .to_string()
                .trim_matches('"')
                .to_string();

            // Extract and parse participants
            let participants_str = point.payload.get("participants")
                .ok_or_else(|| CustomError::MissingEpisodicField("participants"))?
                .to_string();
            let participants: Vec<String> = serde_json::from_str(&participants_str)
                .map_err(|e| CustomError::DatabaseError(format!("Invalid participants format: {}", e)))?;

            VectorMetadata::new_episodic(uuid, content, timestamp, location_text, participants)
        };

        MemoryEntry::new(metadata, vector)
    }

    // Helper: Parse filter conditions from a JSON object.
    fn parse_filter_conditions(&self, filter_json: &serde_json::Value) -> Option<Vec<Condition>> {
        filter_json.get("must").and_then(|v| v.as_array()).map(|must_array| {
            let mut conditions = Vec::new();
            for cond in must_array {
                if let Some(key) = cond.get("key").and_then(|v| v.as_str()) {
                    if let Some(match_obj) = cond.get("match") {
                        if let Some(value_str) = match_obj.get("value").and_then(|v| v.as_str()) {
                            conditions.push(Condition::matches(key, value_str.to_string()));
                        }
                    } else if let Some(range_obj) = cond.get("range") {
                        let gte = range_obj.get("gte").and_then(|v| v.as_f64());
                        let lte = range_obj.get("lte").and_then(|v| v.as_f64());
                        let range = Range {
                            gt: None,
                            gte,
                            lt: None,
                            lte,
                        };
                        conditions.push(Condition::range(key, range));
                    }
                }
            }
            conditions
        })
    }
}

#[async_trait]
impl VectorMemoryRepository for QdrantVectorMemoryRepository {
    async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await?;
        if !collections.collections.iter().any(|c| c.name == self.config.collection_name) {
            let vectors_config = VectorsConfig {
                config: Some(
                    vectors_config::Config::Params(VectorParams {
                        size: self.config.model.vector_size(),
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    }),
                ),
            };

            let create_req = CreateCollectionBuilder::new(&self.config.collection_name)
                .vectors_config(vectors_config)
                .build();
            self.client.create_collection(create_req).await?;
        }
        Ok(())
    }

    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        let memory_value = serde_json::to_value(memory)?;
        let payload = memory_value.as_object()
            .ok_or_else(|| CustomError::DatabaseError("Failed to convert memory to object".to_string()))?
            .clone();

        let point = PointStruct::new(
            memory.id.to_string(),
            memory.embedding.clone(),
            payload,
        );
        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, vec![point]).build();
        self.client.upsert_points(upsert_req).await?;
        Ok(())
    }

    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        self.store_memory(memory).await
    }

    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        let q_id = PointId {
            point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
        };
        let selector = PointsSelectorOneOf::Points(PointsIdsList { ids: vec![q_id] });
        let delete_req = DeletePointsBuilder::new(&self.config.collection_name).points(selector).build();
        self.client.delete_points(delete_req).await?;
        Ok(())
    }

    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        let mut builder = SearchPointsBuilder::new(&self.config.collection_name, query_vector.to_vec(), top_k as u64);
        builder = builder.with_payload(true);

        // Apply filters if present
        if let Some(filters) = filters {
            let filter_json = serde_json::to_value(filters)?;
            if let Some(conditions) = self.parse_filter_conditions(&filter_json) {
                let filter = Filter {
                    must: conditions,
                    should: vec![],
                    must_not: vec![],
                    min_should: None,
                };
                builder = builder.filter(filter);
            }
        }

        let search_req = builder.build();
        let response = self.client.search_points(search_req).await?;
        let results = response.result.into_iter()
            .map(|scored| self.scored_point_to_memory_entry(scored))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        let points: Vec<PointStruct> = memories
            .iter()
            .map(|memory| {
                let memory_value = serde_json::to_value(memory)?;
                let payload = memory_value.as_object()
                    .ok_or_else(|| CustomError::DatabaseError("Failed to convert memory to object".to_string()))?
                    .clone();

                Ok(PointStruct::new(
                    memory.id.to_string(),
                    memory.embedding.clone(),
                    payload,
                ))
            })
            .collect::<Result<_, CustomError>>()?;

        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, points).build();
        self.client.upsert_points(upsert_req).await?;
        Ok(())
    }
}
