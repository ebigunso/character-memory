use uuid::Uuid;
use async_trait::async_trait;

use crate::errors::CustomError;
use crate::models::internal::MemoryEntry;
use crate::models::public::MemoryFilters;

/// Repository trait for storing and retrieving memories using a vector database.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait VectorMemoryRepository: Send + Sync {
    /// Initializes the vector database collection if it doesn't exist.
    async fn init_collection(&self) -> Result<(), CustomError>;

    /// Stores a new memory in the database using upsert semantics.
    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError>;

    /// Updates an existing memory in the database.
    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError>;

    /// Deletes a memory by its ID.
    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError>;

    /// Searches for memories using a query vector and optional filters.
    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<MemoryEntry>, CustomError>;

    /// Inserts multiple memories in a single operation.
    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError>;
}

// Implement the trait for Box<dyn VectorMemoryRepository>
#[async_trait]
impl<T: VectorMemoryRepository + ?Sized> VectorMemoryRepository for Box<T> {
    async fn init_collection(&self) -> Result<(), CustomError> {
        (**self).init_collection().await
    }

    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        (**self).store_memory(memory).await
    }

    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
        (**self).update_memory(memory).await
    }

    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        (**self).delete_memory(id).await
    }

    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        (**self).search_memory(query_vector, top_k, filters).await
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        (**self).bulk_insert(memories).await
    }
}
