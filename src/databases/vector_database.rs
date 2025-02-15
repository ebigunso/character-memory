//! Vector Database trait module.
//! TODO: Abstract qdrant-specific types from the interface.

use async_trait::async_trait;
use qdrant_client::qdrant::{PointStruct, PointsSelector, ScoredPoint, SearchPoints};
use crate::errors::custom::CustomError;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait VectorDatabase {
    /// Lists all collections in the database.
    async fn list_collections(&self) -> Result<Vec<String>, CustomError>;

    /// Creates a new collection with the specified name and vector size.
    async fn create_collection(&self, collection_name: &str, vector_size: u64) -> Result<(), CustomError>;

    /// Inserts or updates points in the specified collection.
    async fn upsert_points(&self, collection_name: &str, points: Vec<PointStruct>) -> Result<(), CustomError>;

    /// Searches for points using the provided search parameters.
    async fn search_points(&self, search: &SearchPoints) -> Result<Vec<ScoredPoint>, CustomError>;

    /// Deletes points from the specified collection using the provided selector.
    async fn delete_points(&self, collection_name: &str, selector: &PointsSelector) -> Result<(), CustomError>;
}
