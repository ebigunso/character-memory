use async_trait::async_trait;
use chrono::{DateTime, Utc};
use qdrant_client::qdrant::{
    point_id::PointIdOptions, points_selector::PointsSelectorOneOf, vector_output, vectors_config,
    vectors_output, Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    DatetimeRange, DeletePointsBuilder, Distance, FieldType, Filter, GetPointsBuilder, PointId,
    PointStruct, PointsIdsList, RetrievedPoint, ScoredPoint, SearchPointsBuilder,
    TextIndexParamsBuilder, Timestamp, TokenizerType, UpsertPointsBuilder, VectorParams,
    VectorsConfig, VectorsOutput,
};
use qdrant_client::{config::QdrantConfig, Qdrant};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::types::{MemoryFilters, MemoryType};
use crate::errors::CustomError;
use crate::internal::config::settings::VectorMemoryRepositorySettings;
use crate::internal::models::memory::{MemoryEntry, ScoredMemoryEntry};
use crate::internal::models::vector::VectorMetadata;
use crate::internal::repositories::VectorMemoryRepository;

#[derive(serde::Serialize)]
struct QdrantMemoryPayload {
    id: Uuid,
    memory_type: MemoryType,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    participants: Option<Vec<String>>,
}

impl From<&MemoryEntry> for QdrantMemoryPayload {
    fn from(memory: &MemoryEntry) -> Self {
        Self {
            id: memory.id,
            memory_type: memory.memory_type.clone(),
            content: memory.content.clone(),
            timestamp: memory.timestamp,
            location_text: memory.location_text.clone(),
            participants: memory.participants.clone(),
        }
    }
}

pub struct QdrantVectorMemoryRepository {
    client: Qdrant,
    config: VectorMemoryRepositorySettings,
}

impl QdrantVectorMemoryRepository {
    pub(crate) fn new(config: VectorMemoryRepositorySettings) -> Result<Self, CustomError> {
        let client = Qdrant::new(QdrantConfig::from_url(&config.url))?;
        Ok(Self { client, config })
    }

    fn build_qdrant_payload(
        memory: &MemoryEntry,
    ) -> Result<serde_json::Map<String, serde_json::Value>, CustomError> {
        let payload_value = serde_json::to_value(QdrantMemoryPayload::from(memory))?;
        payload_value
            .as_object()
            .ok_or_else(|| {
                CustomError::DatabaseError("Failed to convert Qdrant payload to object".to_string())
            })
            .cloned()
    }

    async fn ensure_full_text_indexes(&self) -> Result<(), CustomError> {
        let info = self
            .client
            .collection_info(&self.config.collection_name)
            .await?;

        let empty_payload_schema: HashMap<String, qdrant_client::qdrant::PayloadSchemaInfo> =
            HashMap::new();
        let payload_schema = info
            .result
            .as_ref()
            .map(|r| &r.payload_schema)
            .unwrap_or(&empty_payload_schema);

        let mut missing_fields: Vec<&str> = Vec::new();
        if !payload_schema.contains_key("location_text") {
            missing_fields.push("location_text");
        }
        if !payload_schema.contains_key("participants") {
            missing_fields.push("participants");
        }

        if missing_fields.is_empty() {
            return Ok(());
        }

        let text_index_params = TextIndexParamsBuilder::new(TokenizerType::Multilingual)
            .min_token_len(2)
            .max_token_len(10)
            .lowercase(true)
            .build();

        for field_name in missing_fields {
            self.client
                .create_field_index(
                    CreateFieldIndexCollectionBuilder::new(
                        &self.config.collection_name,
                        field_name,
                        FieldType::Text,
                    )
                    .field_index_params(text_index_params.clone()),
                )
                .await?;
        }

        Ok(())
    }

    // Helper: Convert a Qdrant point to a MemoryEntry.
    fn point_to_memory_entry<P>(&self, point: P) -> Result<MemoryEntry, CustomError>
    where
        P: Into<PointData>,
    {
        self.point_data_to_memory_entry(point.into())
    }

    // Helper: Convert a PointData to a MemoryEntry.
    fn point_data_to_memory_entry(
        &self,
        point_data: PointData,
    ) -> Result<MemoryEntry, CustomError> {
        // Extract ID
        let id = point_data
            .id
            .ok_or_else(|| CustomError::DatabaseError("Missing point ID".to_string()))?;
        let id_str = match id.point_id_options {
            Some(PointIdOptions::Uuid(ref s)) => s.clone(),
            _ => {
                return Err(CustomError::DatabaseError(
                    "Invalid point id variant".to_string(),
                ))
            }
        };
        let uuid = Uuid::parse_str(&id_str)
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {e}")))?;

        // Extract vector
        let vectors_output = point_data
            .vectors
            .ok_or_else(|| CustomError::DatabaseError("Missing vector in point".to_string()))?;
        let vector = match vectors_output.vectors_options {
            Some(vectors_output::VectorsOptions::Vector(vo)) => match vo.into_vector() {
                vector_output::Vector::Dense(vector) => vector.data,
                _ => {
                    return Err(CustomError::DatabaseError(
                        "Unexpected vector output type".to_string(),
                    ))
                }
            },
            _ => {
                return Err(CustomError::DatabaseError(
                    "Unexpected vector type".to_string(),
                ))
            }
        };

        // Extract required fields from payload
        let memory_type = point_data
            .payload
            .get("memory_type")
            .ok_or_else(|| {
                CustomError::DatabaseError("Missing memory_type in payload".to_string())
            })?
            .to_string()
            .trim_matches('"')
            .to_lowercase();

        let content = point_data
            .payload
            .get("content")
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .to_string()
            .trim_matches('"')
            .to_string();

        // Create metadata based on memory type
        let metadata = if memory_type == "semantic" {
            VectorMetadata::new_semantic(uuid, content)
        } else {
            // Extract and parse timestamp
            let timestamp_str = point_data
                .payload
                .get("timestamp")
                .ok_or_else(|| CustomError::MissingEpisodicField("timestamp"))?
                .to_string()
                .trim_matches('"')
                .to_string();
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| CustomError::DatabaseError(format!("Invalid timestamp format: {e}")))?
                .with_timezone(&Utc);

            // Extract location
            let location_text = point_data
                .payload
                .get("location_text")
                .ok_or_else(|| CustomError::MissingEpisodicField("location_text"))?
                .to_string()
                .trim_matches('"')
                .to_string();

            // Extract and parse participants
            let participants_str = point_data
                .payload
                .get("participants")
                .ok_or_else(|| CustomError::MissingEpisodicField("participants"))?
                .to_string();
            let participants: Vec<String> =
                serde_json::from_str(&participants_str).map_err(|e| {
                    CustomError::DatabaseError(format!("Invalid participants format: {e}"))
                })?;

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
            conditions.push(Condition::matches_text(
                "location_text",
                location.to_string(),
            ));
        }

        // Add participants filter if present
        if let Some(participants) = &filters.participants {
            for participant in participants {
                conditions.push(Condition::matches_text(
                    "participants",
                    participant.to_string(),
                ));
            }
        }

        conditions
    }
}

#[async_trait]
impl VectorMemoryRepository for QdrantVectorMemoryRepository {
    async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await?;
        if !collections
            .collections
            .iter()
            .any(|c| c.name == self.config.collection_name)
        {
            let vectors_config = VectorsConfig {
                config: Some(vectors_config::Config::Params(VectorParams {
                    size: self.config.model.vector_size(),
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
            };

            let create_req = CreateCollectionBuilder::new(&self.config.collection_name)
                .vectors_config(vectors_config)
                .build();
            self.client.create_collection(create_req).await?;
        }

        self.ensure_full_text_indexes().await?;
        Ok(())
    }

    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        let payload = Self::build_qdrant_payload(memory)?;

        let point = PointStruct::new(memory.id.to_string(), memory.embedding.clone(), payload);
        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, vec![point])
            .wait(true)
            .build();
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
            return Err(CustomError::DatabaseError(format!(
                "Memory with ID {id} not found",
            )));
        }

        let q_id = PointId {
            point_id_options: Some(PointIdOptions::Uuid(id.to_string())),
        };
        let selector = PointsSelectorOneOf::Points(PointsIdsList { ids: vec![q_id] });
        let delete_req = DeletePointsBuilder::new(&self.config.collection_name)
            .points(selector)
            .build();
        self.client.delete_points(delete_req).await?;
        Ok(())
    }

    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<ScoredMemoryEntry>, CustomError> {
        let mut builder = SearchPointsBuilder::new(
            &self.config.collection_name,
            query_vector.to_vec(),
            top_k as u64,
        );
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
        let results = response
            .result
            .into_iter()
            .map(|scored| {
                let score = scored.score;
                let entry = self.point_to_memory_entry(scored)?;
                Ok(ScoredMemoryEntry { entry, score })
            })
            .collect::<Result<Vec<_>, CustomError>>()?;

        Ok(results)
    }

    async fn get_memories_by_ids<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> Result<Vec<MemoryEntry>, CustomError> {
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
            let found_ids: std::collections::HashSet<Uuid> =
                memories.iter().map(|memory| memory.id).collect();

            let missing_ids: Vec<Uuid> = ids
                .iter()
                .filter(|id| !found_ids.contains(id))
                .cloned()
                .collect();

            if !missing_ids.is_empty() {
                return Err(CustomError::DatabaseError(format!(
                    "Memories with IDs {missing_ids:?} not found",
                )));
            }
        }

        Ok(memories)
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        let points: Vec<PointStruct> = memories
            .iter()
            .map(|memory| {
                let payload = Self::build_qdrant_payload(memory)?;

                Ok(PointStruct::new(
                    memory.id.to_string(),
                    memory.embedding.clone(),
                    payload,
                ))
            })
            .collect::<Result<_, CustomError>>()?;

        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, points)
            .wait(true)
            .build();
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn qdrant_payload_excludes_embedding_semantic() {
        let id = Uuid::new_v4();
        let metadata = VectorMetadata::new_semantic(id, "Alice is a software engineer".to_string());
        let entry = MemoryEntry::new(metadata, vec![0.1, 0.2]).expect("valid semantic memory");

        let payload =
            QdrantVectorMemoryRepository::build_qdrant_payload(&entry).expect("payload builds");

        assert!(payload.get("embedding").is_none());
        assert!(payload.get("id").is_some());
        assert!(payload.get("memory_type").is_some());
        assert!(payload.get("content").is_some());

        assert!(payload.get("timestamp").is_none());
        assert!(payload.get("location_text").is_none());
        assert!(payload.get("participants").is_none());
    }

    #[test]
    fn qdrant_payload_excludes_embedding_episodic() {
        let id = Uuid::new_v4();
        let timestamp = Utc.with_ymd_and_hms(2025, 2, 2, 14, 0, 0).unwrap();
        let metadata = VectorMetadata::new_episodic(
            id,
            "Discussed weekend plans".to_string(),
            timestamp,
            "Café Central".to_string(),
            vec!["Alice".to_string(), "Bob".to_string()],
        );
        let entry = MemoryEntry::new(metadata, vec![0.1, 0.2]).expect("valid episodic memory");

        let payload =
            QdrantVectorMemoryRepository::build_qdrant_payload(&entry).expect("payload builds");

        assert!(payload.get("embedding").is_none());
        assert!(payload.get("id").is_some());
        assert!(payload.get("memory_type").is_some());
        assert!(payload.get("content").is_some());

        assert!(payload.get("timestamp").is_some());
        assert!(payload.get("location_text").is_some());
        assert!(payload.get("participants").is_some());
    }
}
