use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use qdrant_client::qdrant::{
    point_id::PointIdOptions, points_selector::PointsSelectorOneOf, CreateCollectionBuilder,
    DeletePointsBuilder, Distance, PointStruct, PointsIdsList, Range, ScoredPoint,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParams, VectorsConfig, PointId,
    Filter, Condition, vectors_config, vectors_output, VectorsOutput, GetPointsBuilder, RetrievedPoint,
    DatetimeRange, Timestamp,
};
use qdrant_client::{Qdrant, config::QdrantConfig};
use uuid::Uuid;

use crate::config::settings::VectorMemoryRepositorySettings;
use crate::errors::CustomError;
use crate::models::memory::MemoryEntry;
use crate::models::vector::VectorMetadata;
use crate::models::memory::dto::MemoryFilters;
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

    // Helper: Convert a Qdrant point to a MemoryEntry.
    fn point_to_memory_entry<P>(&self, point: P) -> Result<MemoryEntry, CustomError>
    where
        P: Into<PointData>,
    {
        self.point_data_to_memory_entry(point.into())
    }

    // Helper: Convert a PointData to a MemoryEntry.
    fn point_data_to_memory_entry(&self, point_data: PointData) -> Result<MemoryEntry, CustomError> {
        // Extract ID
        let id = point_data.id.ok_or_else(|| CustomError::DatabaseError("Missing point ID".to_string()))?;
        let id_str = match id.point_id_options {
            Some(PointIdOptions::Uuid(ref s)) => s.clone(),
            _ => return Err(CustomError::DatabaseError("Invalid point id variant".to_string())),
        };
        let uuid = Uuid::parse_str(&id_str)
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {}", e)))?;

        // Extract vector
        let vectors_output = point_data.vectors.ok_or_else(|| CustomError::DatabaseError("Missing vector in point".to_string()))?;
        let vector = match vectors_output.vectors_options {
            Some(vectors_output::VectorsOptions::Vector(vo)) => vo.data,
            _ => return Err(CustomError::DatabaseError("Unexpected vector type".to_string())),
        };

        // Extract required fields from payload
        let memory_type = point_data.payload.get("memory_type")
            .ok_or_else(|| CustomError::DatabaseError("Missing memory_type in payload".to_string()))?
            .to_string()
            .trim_matches('"')
            .to_lowercase();

        let content = point_data.payload.get("content")
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .to_string()
            .trim_matches('"')
            .to_string();

        // Create metadata based on memory type
        let metadata = if memory_type == "semantic" {
            VectorMetadata::new_semantic(uuid, content)
        } else {
            // Extract and parse timestamp
            let timestamp_str = point_data.payload.get("timestamp")
                .ok_or_else(|| CustomError::MissingEpisodicField("timestamp"))?
                .to_string()
                .trim_matches('"')
                .to_string();
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| CustomError::DatabaseError(format!("Invalid timestamp format: {}", e)))?
                .with_timezone(&Utc);

            // Extract location
            let location_text = point_data.payload.get("location_text")
                .ok_or_else(|| CustomError::MissingEpisodicField("location_text"))?
                .to_string()
                .trim_matches('"')
                .to_string();

            // Extract and parse participants
            let participants_str = point_data.payload.get("participants")
                .ok_or_else(|| CustomError::MissingEpisodicField("participants"))?
                .to_string();
            let participants: Vec<String> = serde_json::from_str(&participants_str)
                .map_err(|e| CustomError::DatabaseError(format!("Invalid participants format: {}", e)))?;

            VectorMetadata::new_episodic(uuid, content, timestamp, location_text, participants)
        };

        MemoryEntry::new(metadata, vector)
    }

    // Helper: Parse filter conditions from MemoryFilters.
    fn parse_filter_conditions(&self, filters: &MemoryFilters) -> Vec<Condition> {
        let mut conditions = Vec::new();

        // Add memory_type filter if present
        if let Some(memory_type) = &filters.memory_type {
            conditions.push(Condition::matches("memory_type", memory_type.to_string()));
        }

        // Add date range filter if start_date or end_date is present
        if filters.start_date.is_some() || filters.end_date.is_some() {
            let timestamp_field = "timestamp";

            // Create a DatetimeRange with appropriate bounds
            let mut datetime_range = DatetimeRange::default();

            // Set lower bound if start_date is present
            if let Some(start_date) = filters.start_date {
                // Convert chrono DateTime to Timestamp
                // Timestamp expects seconds since epoch
                let seconds = start_date.timestamp();
                let nanos = start_date.timestamp_subsec_nanos();

                datetime_range.gte = Some(Timestamp {
                    seconds,
                    nanos: nanos as i32,
                });
            }

            // Set upper bound if end_date is present
            if let Some(end_date) = filters.end_date {
                // Convert chrono DateTime to Timestamp
                let seconds = end_date.timestamp();
                let nanos = end_date.timestamp_subsec_nanos();

                datetime_range.lte = Some(Timestamp {
                    seconds,
                    nanos: nanos as i32,
                });
            }

            // Add the datetime range condition
            conditions.push(Condition::datetime_range(timestamp_field, datetime_range));
        }

        // Add location_text filter if present
        if let Some(location) = &filters.location_text {
            conditions.push(Condition::matches("location_text", location.to_string()));
        }

        // Add participants filter if present
        if let Some(participants) = &filters.participants {
            for participant in participants {
                conditions.push(Condition::matches("participants", format!("*{}*", participant)));
            }
        }

        conditions
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
        // First check if the memory exists
        let memories = self.get_memories_by_ids(&[id]).await;
        if memories.is_err() {
            return Err(CustomError::DatabaseError(format!("Memory with ID {} not found", id)));
        }

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
        builder = builder.with_vectors(true);

        // Apply filters if present
        if let Some(filters) = filters {
            let conditions = self.parse_filter_conditions(filters);
            if !conditions.is_empty() {
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
            .map(|scored| self.point_to_memory_entry(scored))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    async fn get_memories_by_ids<'a>(&'a self, ids: &'a [Uuid]) -> Result<Vec<MemoryEntry>, CustomError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Convert UUIDs to Qdrant PointIds
        let point_ids: Vec<PointId> = ids
            .iter()
            .map(|id| PointId {
                point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
            })
            .collect();

        // Build the get points request
        let get_points_request = GetPointsBuilder::new(&self.config.collection_name, point_ids)
            .with_payload(true)
            .with_vectors(true)
            .build();

        // Execute the request
        let response = self.client.get_points(get_points_request).await?;

        // Convert the points to MemoryEntry objects
        let memories = response
            .result
            .into_iter()
            .map(|point| self.point_to_memory_entry(point))
            .collect::<Result<Vec<_>, _>>()?;

        // Check if all requested IDs were found
        if memories.len() < ids.len() {
            // Find which IDs were not found
            let found_ids: std::collections::HashSet<Uuid> = memories.iter()
                .map(|memory| memory.id)
                .collect();

            let missing_ids: Vec<Uuid> = ids.iter()
                .filter(|id| !found_ids.contains(id))
                .cloned()
                .collect();

            if !missing_ids.is_empty() {
                return Err(CustomError::DatabaseError(
                    format!("Memories with IDs {:?} not found", missing_ids)
                ));
            }
        }

        Ok(memories)
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

// Helper struct to unify ScoredPoint and RetrievedPoint
struct PointData {
    id: Option<PointId>,
    payload: HashMap<String, qdrant_client::qdrant::Value>,
    vectors: Option<VectorsOutput>,
}

impl PointData {
    fn from_scored_point(point: ScoredPoint) -> Self {
        Self {
            id: point.id,
            payload: point.payload,
            vectors: point.vectors,
        }
    }

    fn from_retrieved_point(point: RetrievedPoint) -> Self {
        Self {
            id: point.id,
            payload: point.payload,
            vectors: point.vectors,
        }
    }
}

impl From<ScoredPoint> for PointData {
    fn from(point: ScoredPoint) -> Self {
        Self::from_scored_point(point)
    }
}

impl From<RetrievedPoint> for PointData {
    fn from(point: RetrievedPoint) -> Self {
        Self::from_retrieved_point(point)
    }
}
