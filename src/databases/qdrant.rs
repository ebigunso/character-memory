use async_trait::async_trait;
use qdrant_client::qdrant::{
        point_id::PointIdOptions, points_selector::PointsSelectorOneOf, CreateCollectionBuilder, DeletePointsBuilder, Distance, PointStruct, PointsIdsList, Range, ScoredPoint, SearchPointsBuilder, UpsertPointsBuilder, VectorParams, VectorsConfig
    };
use qdrant_client::Qdrant;
use crate::errors::custom::CustomError;
use crate::databases::domain_types::{DbPoint, DbSearchQuery, DbSearchResult};
use crate::databases::vector_database::VectorDatabase;

pub(crate) struct QdrantDatabaseImpl(Qdrant);

impl QdrantDatabaseImpl {
    pub fn new(client: Qdrant) -> Self {
        Self(client)
    }

    // Helper: Convert a DbPoint to a Qdrant PointStruct.
    fn convert_db_point(&self, db_point: DbPoint) -> PointStruct {
        let q_id = db_point.id.map(|s| {
            qdrant_client::qdrant::PointId {
                point_id_options: Some(PointIdOptions::Uuid(s)),
            }
        });
        let payload = db_point.payload.into_iter().map(|(k, v)| {
            (k, qdrant_client::qdrant::Value::from(v.to_string()))
        }).collect();
        PointStruct {
            id: q_id,
            payload,
            vectors: Some(db_point.vector.into()),
        }
    }

    // Helper: Convert a Qdrant ScoredPoint to a DbSearchResult.
    fn convert_scored_point(&self, point: ScoredPoint) -> Result<DbSearchResult, CustomError> {
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
        Ok(DbSearchResult {
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
impl VectorDatabase for QdrantDatabaseImpl {
    async fn list_collections(&self) -> Result<Vec<String>, CustomError> {
        let collections = self.0.list_collections().await?;
        Ok(collections.collections.into_iter().map(|c| c.name).collect())
    }

    async fn create_collection(&self, collection_name: &str, vector_size: u64) -> Result<(), CustomError> {
        let vectors_config = VectorsConfig {
            config: Some(
                qdrant_client::qdrant::vectors_config::Config::Params(VectorParams {
                    size: vector_size,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                }),
            ),
        };

        let create_req = CreateCollectionBuilder::new(collection_name)
            .vectors_config(vectors_config)
            .build();
        self.0.create_collection(create_req).await?;
        Ok(())
    }

    async fn upsert_points(&self, collection_name: &str, points: Vec<DbPoint>) -> Result<(), CustomError> {
        let q_points: Vec<PointStruct> = points.into_iter().map(|p| self.convert_db_point(p)).collect();
        let upsert_req = UpsertPointsBuilder::new(collection_name, q_points).build();
        self.0.upsert_points(upsert_req).await?;
        Ok(())
    }

    async fn search_points(&self, query: &DbSearchQuery) -> Result<Vec<DbSearchResult>, CustomError> {
        let mut builder = SearchPointsBuilder::new(&query.collection_name, query.vector.clone(), query.limit);
        builder = builder.with_payload(query.with_payload);

        // Apply filters if present.
        if let Some(filter_json) = &query.filter {
            if let Some(conditions) = self.parse_filter_conditions(filter_json) {
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
        let response = self.0.search_points(search_req).await?;
        let mut results = Vec::new();
        for scored in response.result {
            results.push(self.convert_scored_point(scored)?);
        }
        Ok(results)
    }

    async fn delete_points(&self, collection_name: &str, point_ids: Vec<String>) -> Result<(), CustomError> {
        let q_ids: Vec<_> = point_ids.into_iter().map(|s| {
            qdrant_client::qdrant::PointId {
                point_id_options: Some(PointIdOptions::Uuid(s)),
            }
        }).collect();
        let selector = PointsSelectorOneOf::Points(PointsIdsList { ids: q_ids });
        let delete_req = DeletePointsBuilder::new(collection_name).points(selector).build();
        self.0.delete_points(delete_req).await?;
        Ok(())
    }
}
