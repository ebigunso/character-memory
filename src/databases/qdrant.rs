use async_trait::async_trait;
use qdrant_client::{
    qdrant::{
        vectors_config::Config, Distance, PointStruct, SearchPoints, ScoredPoint, VectorParams, VectorsConfig, PointsSelector,
        CreateCollectionBuilder, UpsertPointsBuilder, DeletePointsBuilder, SearchPointsBuilder,
    },
    Qdrant
};
use crate::errors::custom::CustomError;
use super::vector_database::VectorDatabase;

pub(crate) struct QdrantDatabaseImpl(Qdrant);

impl QdrantDatabaseImpl {
    pub fn new(client: Qdrant) -> Self {
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

        let create_req = CreateCollectionBuilder::new(collection_name)
            .vectors_config(vectors_config)
            .build();
        self.0.create_collection(create_req).await?;

        Ok(())
    }

    async fn upsert_points(&self, collection_name: &str, points: Vec<PointStruct>) -> Result<(), CustomError> {
        let upsert_req = UpsertPointsBuilder::new(collection_name, points)
            .build();
        self.0.upsert_points(upsert_req).await?;
        Ok(())
    }

    async fn search_points(&self, search: &SearchPoints) -> Result<Vec<ScoredPoint>, CustomError> {
        let search_req = SearchPointsBuilder::new(
            &search.collection_name,
            search.vector.clone(),
            search.limit,
        )
        .with_payload(
            search.with_payload
                .as_ref()
                .and_then(|s| s.selector_options.clone())
                .unwrap_or_else(|| true.into())
        )
        .build();

        let response = self.0.search_points(search_req).await?;

        Ok(response.result)
    }

    async fn delete_points(&self, collection_name: &str, selector: &PointsSelector) -> Result<(), CustomError> {
        let points_selector = selector
            .clone()
            .points_selector_one_of
            .ok_or_else(|| CustomError::DatabaseError("points_selector_one_of must be specified for delete operation".to_string()))?;

        let delete_req = DeletePointsBuilder::new(collection_name)
            .points(points_selector)
            .build();
        self.0.delete_points(delete_req).await?;
        Ok(())
    }
}
