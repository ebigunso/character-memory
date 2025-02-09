use async_trait::async_trait;
use qdrant_client::qdrant::{PointStruct, PointsSelector, ScoredPoint, SearchPoints};
use crate::errors::custom::CustomError;

// ToDo: This still uses qdrant specific types, we need to abstract them away

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait VectorDatabase {
    /// List all collections in the database
    async fn list_collections(&self) -> Result<Vec<String>, CustomError>;

    /// Create a new collection with the specified name and vector size
    async fn create_collection(&self, collection_name: &str, vector_size: u64) -> Result<(), CustomError>;

    /// Insert or update points in the specified collection
    async fn upsert_points(&self, collection_name: &str, points: Vec<PointStruct>) -> Result<(), CustomError>;

    /// Search for points using the provided search parameters
    async fn search_points(&self, search: &SearchPoints) -> Result<Vec<ScoredPoint>, CustomError>;

    /// Delete points from the specified collection using the provided selector
    async fn delete_points(&self, collection_name: &str, selector: &PointsSelector) -> Result<(), CustomError>;
}
