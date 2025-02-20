use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use crate::databases::vector_database::VectorDatabase;
use crate::databases::domain_types::{DbPoint, DbSearchQuery};
use crate::errors::custom::CustomError;
use crate::models::internal::{MemoryEntry, MemoryType};
use crate::models::public::MemoryFilters;
use crate::repositories::vector_memory_repository::{VectorMemoryRepository, VectorMemoryConfig};

pub struct QdrantVectorMemoryRepository {
    client: Box<dyn VectorDatabase + Send + Sync>,
    config: VectorMemoryConfig,
}

impl QdrantVectorMemoryRepository {
    pub fn new(client: Box<dyn VectorDatabase + Send + Sync>, config: VectorMemoryConfig) -> Self {
        Self { client, config }
    }

    // Helper: Convert a MemoryEntry into a domain DbPoint.
    fn memory_to_db_point(memory: &MemoryEntry) -> DbPoint {
        let mut payload_map = HashMap::new();
        payload_map.insert(
            "memory_type".to_string(),
            serde_json::Value::String(format!("{:?}", memory.memory_type).to_lowercase()),
        );
        payload_map.insert(
            "content".to_string(),
            serde_json::Value::String(memory.content.clone()),
        );
        if let Some(timestamp) = &memory.timestamp {
            payload_map.insert(
                "timestamp".to_string(),
                serde_json::Value::Number(serde_json::Number::from(timestamp.timestamp())),
            );
        }
        if let Some(location) = &memory.location_text {
            payload_map.insert(
                "location_text".to_string(),
                serde_json::Value::String(location.clone()),
            );
        }
        if let Some(participants) = &memory.participants {
            payload_map.insert(
                "participants".to_string(),
                serde_json::Value::Array(
                    participants.iter().map(|p| serde_json::Value::String(p.clone())).collect()
                ),
            );
        }
        DbPoint {
            id: Some(memory.id.to_string()),
            payload: payload_map,
            vector: memory.embedding.clone(),
        }
    }

    // Helper: Convert a domain DbSearchResult into a MemoryEntry.
    fn scored_point_to_memory(result: &crate::databases::domain_types::DbSearchResult) -> Result<MemoryEntry, CustomError> {
        let payload = &result.payload;
        let memory_type_str = payload.get("memory_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CustomError::DatabaseError("Missing memory_type in payload".to_string()))?;
        let memory_type = match memory_type_str {
            "episodic" => MemoryType::Episodic,
            _ => MemoryType::Semantic,
        };
        let content = payload.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .to_string();
        let embedding = result.vector.clone();
        let id = Uuid::parse_str(&result.id)
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {}", e)))?;
        let timestamp = payload.get("timestamp")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        let location_text = payload.get("location_text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let participants = payload.get("participants")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_owned()))
                    .collect::<Vec<_>>()
            });
        Ok(MemoryEntry {
            id,
            memory_type,
            content,
            embedding,
            timestamp,
            location_text,
            participants,
        })
    }
}

#[async_trait]
impl VectorMemoryRepository for QdrantVectorMemoryRepository {
    async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await?;
        if collections.iter().any(|name| name == &self.config.collection_name) {
            return Ok(());
        }
        self.client.create_collection(&self.config.collection_name, self.config.vector_size).await
    }

    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        if memory.embedding.len() != self.config.vector_size as usize {
            return Err(CustomError::MemoryValidation(format!(
                "Embedding size mismatch. Expected {}, got {}",
                self.config.vector_size,
                memory.embedding.len()
            )));
        }
        memory.validate()?;
        let db_point = Self::memory_to_db_point(memory);
        self.client.upsert_points(&self.config.collection_name, vec![db_point]).await
    }

    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        self.store_memory(memory).await
    }

    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.client
            .delete_points(&self.config.collection_name, vec![id.to_string()])
            .await
    }

    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        if query_vector.len() != self.config.vector_size as usize {
            return Err(CustomError::MemoryValidation(format!(
                "Query vector size mismatch. Expected {}, got {}",
                self.config.vector_size,
                query_vector.len()
            )));
        }

        let filter = if let Some(filters) = filters {
            let mut must_conditions = Vec::new();
            if let Some(mem_type) = &filters.memory_type {
                must_conditions.push(serde_json::json!({
                    "key": "memory_type",
                    "match": { "value": mem_type.to_lowercase() }
                }));
            }
            if filters.start_date.is_some() || filters.end_date.is_some() {
                let mut range_obj = serde_json::Map::new();
                if let Some(start) = filters.start_date {
                    range_obj.insert("gte".to_string(), serde_json::json!(start.timestamp()));
                }
                if let Some(end) = filters.end_date {
                    range_obj.insert("lte".to_string(), serde_json::json!(end.timestamp()));
                }
                must_conditions.push(serde_json::json!({
                    "key": "timestamp",
                    "range": range_obj
                }));
            }
            Some(serde_json::json!({
                "must": must_conditions
            }))
        } else {
            None
        };

        let db_query = DbSearchQuery {
            collection_name: self.config.collection_name.clone(),
            vector: query_vector.to_vec(),
            limit: top_k as u64,
            filter,
            with_payload: true,
        };
        let results = self.client.search_points(&db_query).await?;
        let mut memories = Vec::with_capacity(results.len());
        for res in results {
            memories.push(Self::scored_point_to_memory(&res)?);
        }
        Ok(memories)
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        let mut points = Vec::new();
        for memory in memories {
            if memory.embedding.len() != self.config.vector_size as usize {
                return Err(CustomError::MemoryValidation(format!(
                    "Embedding size mismatch. Expected {}, got {}",
                    self.config.vector_size,
                    memory.embedding.len()
                )));
            }
            memory.validate()?;
            points.push(Self::memory_to_db_point(memory));
        }
        self.client.upsert_points(&self.config.collection_name, points).await
    }
}
