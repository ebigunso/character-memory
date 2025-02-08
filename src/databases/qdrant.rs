use async_trait::async_trait;
use qdrant_client::{
    prelude::*,
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointStruct, SearchPoints,
        VectorParams, VectorsConfig, PointsSelector,
    },
};
use crate::errors::custom::CustomError;
use super::vector_database::VectorDatabase;

pub(crate) struct QdrantDatabaseImpl(QdrantClient);

impl QdrantDatabaseImpl {
    pub fn new(client: QdrantClient) -> Self {
        Self(client)
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
            config: Some(Config::Params(VectorParams {
                size: vector_size,
                distance: Distance::Cosine.into(),
                ..Default::default()
            })),
        };

        self.0.create_collection(&CreateCollection {
            collection_name: collection_name.to_string(),
            vectors_config: Some(vectors_config),
            ..Default::default()
        })
        .await?;

        Ok(())
    }

    async fn upsert_points(&self, collection_name: &str, points: Vec<PointStruct>) -> Result<(), CustomError> {
        self.0.upsert_points(collection_name, points, None).await?;
        Ok(())
    }

    async fn search_points(&self, search: &SearchPoints) -> Result<Vec<PointStruct>, CustomError> {
        let result = self.0.search_points(search).await?;
        Ok(result.result)
    }

    async fn delete_points(&self, collection_name: &str, selector: &PointsSelector) -> Result<(), CustomError> {
        self.0.delete_points(collection_name, selector, None).await?;
        Ok(())
    }
}
