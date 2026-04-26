use async_trait::async_trait;
use uuid::Uuid;

use crate::api::types::MemoryFilters;
use crate::errors::CustomError;
use crate::internal::models::memory::{MemoryEntry, ScoredMemoryEntry};

/// Repository trait for storing and retrieving memories using a vector database.
///
/// # Description
///
/// This trait defines the interface for vector database operations.
/// Implementations handle the storage and retrieval of memory entries
/// in a vector database, supporting semantic search capabilities.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait VectorMemoryRepository: Send + Sync {
    /// Initializes the vector database collection if it doesn't exist.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if initialization succeeds
    /// - `Err`: A `CustomError` if initialization fails
    async fn init_collection(&self) -> Result<(), CustomError>;

    /// Stores a new memory in the database using upsert semantics.
    ///
    /// # Parameters
    ///
    /// - `memory`: The `MemoryEntry` to store in the database
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if storage succeeds
    /// - `Err`: A `CustomError` if storage fails
    async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError>;

    /// Updates an existing memory in the database.
    ///
    /// # Parameters
    ///
    /// - `memory`: The `MemoryEntry` containing updated data
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if update succeeds
    /// - `Err`: A `CustomError` if update fails
    async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError>;

    /// Deletes a memory by its ID.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to delete
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if deletion succeeds
    /// - `Err`: A `CustomError` if deletion fails
    async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError>;

    /// Searches for memories using a query vector and optional filters.
    ///
    /// # Parameters
    ///
    /// - `query_vector`: The vector to use for similarity search
    /// - `top_k`: The maximum number of results to return
    /// - `filters`: Optional filters to apply to the search results
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the search results
    /// - `Err`: A `CustomError` if search fails
    async fn search_memory<'a>(
        &'a self,
        query_vector: &'a [f32],
        top_k: usize,
        filters: Option<&'a MemoryFilters>,
    ) -> Result<Vec<ScoredMemoryEntry>, CustomError>;

    /// Inserts multiple memories in a single operation.
    ///
    /// # Parameters
    ///
    /// - `memories`: A slice of `MemoryEntry` to insert
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if bulk insert succeeds
    /// - `Err`: A `CustomError` if bulk insert fails
    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError>;

    /// Retrieves multiple memories by their IDs.
    ///
    /// # Parameters
    ///
    /// - `ids`: A slice of UUIDs to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the retrieved memories.
    ///   If any requested ID is not found, implementations should return an error rather than an empty or partial result.
    /// - `Err`: A `CustomError` if retrieval fails
    async fn get_memories_by_ids<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> Result<Vec<MemoryEntry>, CustomError>;
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
    ) -> Result<Vec<ScoredMemoryEntry>, CustomError> {
        (**self).search_memory(query_vector, top_k, filters).await
    }

    async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
        (**self).bulk_insert(memories).await
    }

    async fn get_memories_by_ids<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        (**self).get_memories_by_ids(ids).await
    }
}
