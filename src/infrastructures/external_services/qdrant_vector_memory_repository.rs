use async_trait::async_trait;
use qdrant_client::qdrant::{
    point_id::PointIdOptions, points_selector::PointsSelectorOneOf, CreateCollectionBuilder,
    DeletePointsBuilder, Distance, PointStruct, PointsIdsList, Range, ScoredPoint,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParams, VectorsConfig
};
use qdrant_client::Qdrant;
use serde_json::{Value, Map};
use uuid::Uuid;

use crate::errors::custom::CustomError;
use crate::models::domain::{Point, SearchQuery, SearchResult};
use crate::models::internal::MemoryEntry;
use crate::models::public::MemoryFilters;
use crate::repositories::vector_memory_repository::{VectorMemoryConfig, VectorMemoryRepository};

pub struct QdrantVectorMemoryRepository {
    client: Qdrant,
    config: VectorMemoryConfig,
}

impl QdrantVectorMemoryRepository {
    pub(crate) fn from_config(config: VectorMemoryConfig) -> Result<Self, CustomError> {
        let client = Qdrant::new(
            qdrant_client::config::QdrantConfig::from_url(&config.url)
        )?;
        Ok(Self { client, config })
    }

    fn new(client: Qdrant, config: VectorMemoryConfig) -> Self {
        Self { client, config }
    }

    // Helper: Convert a Point to a Qdrant PointStruct.
    fn convert_point(&self, point: Point) -> PointStruct {
        let q_id = point.id.map(|s| {
            qdrant_client::qdrant::PointId {
                point_id_options: Some(PointIdOptions::Uuid(s)),
            }
        });
        let payload = point.payload.into_iter().map(|(k, v)| {
            (k, qdrant_client::qdrant::Value::from(v.to_string()))
        }).collect();
        PointStruct {
            id: q_id,
            payload,
            vectors: Some(point.vector.into()),
        }
    }

    // Helper: Convert a Qdrant ScoredPoint to a SearchResult.
    fn convert_scored_point(&self, point: ScoredPoint) -> Result<SearchResult, CustomError> {
        let id = point.id.ok_or_else(|| CustomError::DatabaseError("Missing point ID".to_string()))?;
        let id_str = match id.point_id_options {
            Some(PointIdOptions::Uuid(ref s)) => s.clone(),
            _ => return Err(CustomError::DatabaseError("Invalid point id variant".to_string())),
        };
        let payload = point.payload.into_iter().map(|(k, v)| {
            (k, serde_json::Value::String(v.to_string()))
        }).collect();
        let vectors_output = point.vectors.ok_or_else(|| CustomError::DatabaseError("Missing vector in point".to_string()))?;
        let vector = match vectors_output.vectors_options {
            Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(vo)) => vo.data,
            _ => return Err(CustomError::DatabaseError("Unexpected vector type".to_string())),
        };
        Ok(SearchResult {
            id: id_str,
            payload,
            vector,
            score: point.score,
        })
    }

    // Helper: Parse filter conditions from a JSON object.
    fn parse_filter_conditions(&self, filter_json: &serde_json::Value) -> Option<Vec<qdrant_client::qdrant::Condition>> {
        filter_json.get("must").and_then(|v| v.as_array()).map(|must_array| {
            let mut conditions = Vec::new();
            for cond in must_array {
                if let Some(key) = cond.get("key").and_then(|v| v.as_str()) {
                    if let Some(match_obj) = cond.get("match") {
                        if let Some(value_str) = match_obj.get("value").and_then(|v| v.as_str()) {
                            conditions.push(qdrant_client::qdrant::Condition::matches(key, value_str.to_string()));
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
                        conditions.push(qdrant_client::qdrant::Condition::range(key, range));
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
                    qdrant_client::qdrant::vectors_config::Config::Params(VectorParams {
                        size: self.config.vector_size,
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

        let point = Point {
            id: Some(memory.id.to_string()),
            payload: payload.into_iter().collect(),
            vector: memory.embedding.clone(),
        };
        let q_point = self.convert_point(point);
        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, vec![q_point]).build();
        self.client.upsert_points(upsert_req).await?;
        Ok(())
    }

    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        self.store_memory(memory).await
    }

    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        let q_id = qdrant_client::qdrant::PointId {
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
                let filter = qdrant_client::qdrant::Filter {
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
        let mut results = Vec::new();
        for scored in response.result {
            let search_result = self.convert_scored_point(scored)?;
            let memory_value = Value::Object(search_result.payload);
            let memory: MemoryEntry = serde_json::from_value(memory_value)?;
            results.push(memory);
        }
        Ok(results)
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        let points: Vec<Point> = memories
            .iter()
            .map(|memory| {
                let memory_value = serde_json::to_value(memory)?;
                let payload = memory_value.as_object()
                    .ok_or_else(|| CustomError::DatabaseError("Failed to convert memory to object".to_string()))?
                    .clone();

                Ok(Point {
                    id: Some(memory.id.to_string()),
                    payload: payload.into_iter().collect(),
                    vector: memory.embedding.clone(),
                })
            })
            .collect::<Result<_, CustomError>>()?;

        let q_points: Vec<PointStruct> = points.into_iter().map(|p| self.convert_point(p)).collect();
        let upsert_req = UpsertPointsBuilder::new(&self.config.collection_name, q_points).build();
        self.client.upsert_points(upsert_req).await?;
        Ok(())
    }
}
