use async_trait::async_trait;
use qdrant_client::{
    qdrant::{
        vectors_config::Config, Distance, PointStruct, SearchPoints,
        VectorParams, VectorsConfig, PointsIdsList, PointsSelector, points_selector::PointsSelectorOneOf,
        CreateCollectionBuilder, UpsertPointsBuilder, DeletePointsBuilder, SearchPointsBuilder,
    },
    Qdrant,
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

    async fn search_points(&self, search: &SearchPoints) -> Result<Vec<PointStruct>, CustomError> {
        let search_req = SearchPointsBuilder::new(
            &search.collection_name,
            search.vector.clone(),
            search.limit
        )
        .with_payload(search.with_payload.unwrap_or(true))
        .build();
        let response = self.0.search_points(search_req).await?;
        let points: Vec<PointStruct> = response.result
            .into_iter()
            .map(|sp| PointStruct {
                id: sp.id,
                payload: sp.payload,
                vectors: sp.vectors.map(|v| match v {
                    qdrant_client::qdrant::Vectors::Simple(vec) => vec,
                    _ => panic!("Unexpected vector variant encountered in search response"),
                }),
                ..Default::default()
            })
            .collect();
        Ok(points)
    }

    async fn delete_points(&self, collection_name: &str, selector: &PointsSelector) -> Result<(), CustomError> {
        let delete_req = DeletePointsBuilder::new(collection_name)
            .points::<PointsSelectorOneOf>(
                PointsIdsList::from(selector.clone())
            )
            .build();
        self.0.delete_points(delete_req).await?;
        Ok(())
    }
}
