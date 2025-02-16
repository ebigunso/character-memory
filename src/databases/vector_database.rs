use async_trait::async_trait;
use crate::errors::custom::CustomError;
use crate::databases::domain_types::{DbPoint, DbSearchQuery, DbSearchResult};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait VectorDatabase {
    async fn list_collections(&self) -> Result<Vec<String>, CustomError>;
    async fn create_collection(&self, collection_name: &str, vector_size: u64) -> Result<(), CustomError>;
    async fn upsert_points(&self, collection_name: &str, points: Vec<DbPoint>) -> Result<(), CustomError>;
    async fn search_points(&self, query: &DbSearchQuery) -> Result<Vec<DbSearchResult>, CustomError>;
    async fn delete_points(&self, collection_name: &str, point_ids: Vec<String>) -> Result<(), CustomError>;
}
