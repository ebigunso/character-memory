use async_trait::async_trait;
use qdrant_client::{
    Qdrant,
    qdrant::{
        CreateCollectionBuilder, DeletePointsBuilder, Distance, PointStruct,
        SearchPointsBuilder, ScoredPoint, UpsertPointsBuilder, VectorParams, VectorsConfig,
        PointsSelector,
        points_selector::PointsSelectorOneOf,
        point_id::PointIdOptions,
        PointsIdsList,
    },
};
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
        // Note: Filter conversion is omitted for simplicity.
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
