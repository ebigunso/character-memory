use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::memory::dto::MemoryFilters;
use crate::models::memory::MemoryEntry;
use crate::models::vector::VectorMetadata;
use crate::repositories::{EmbeddingRepository, VectorMemoryRepository};

/// Provides high-level operations for managing memory entries.
///
/// # Description
///
/// Delegates embedding generation to an EmbeddingRepository and storage/retrieval to a
/// VectorMemoryRepository. All dependencies are injected as arguments.
///
/// # See also
///
/// - [`EmbeddingRepository`]
/// - [`VectorMemoryRepository`]
pub(crate) struct MemoryRepository {
    pub embed_repo: Box<dyn EmbeddingRepository>,
    pub vector_repo: Box<dyn VectorMemoryRepository>,
}

impl MemoryRepository {
    /// Creates a new MemoryRepository instance.
    ///
    /// # Parameters
    ///
    /// - `embed_repo`: The embedding repository to generate embeddings
    /// - `vector_repo`: The vector memory repository for storing memory entries
    ///
    /// # Returns
    ///
    /// A new `MemoryRepository` instance with the provided repositories
    pub fn new(embed_repo: Box<dyn EmbeddingRepository>, vector_repo: Box<dyn VectorMemoryRepository>) -> Self {
        Self { embed_repo, vector_repo }
    }

    /// Initializes the underlying storage systems.
    ///
    /// # Description
    ///
    /// Ensures all required storage systems are properly initialized before any operations are performed.
    /// This should be called during application startup.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if initialization succeeds
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn init_storage(&self) -> Result<(), CustomError> {
        self.vector_repo.init_collection().await
    }

    /// Creates a new memory entry.
    ///
    /// # Description
    ///
    /// Generates an embedding for the input content, validates and constructs a MemoryEntry,
    /// and persists the memory entry via the vector repository.
    ///
    /// # Parameters
    ///
    /// - `input`: A `MemoryInput` containing the data for the new memory entry
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `MemoryEntry` containing the created memory
    /// - `Err`: A `CustomError` if embedding generation, memory validation, or storage fails
    pub async fn create_memory(&self, metadata: VectorMetadata) -> Result<MemoryEntry, CustomError> {
        let embedding = self.embed_repo.generate_embedding(&metadata.content)?;
        let entry = MemoryEntry::new(metadata, embedding)?;
        self.vector_repo.store_memory(&entry).await?;

        Ok(entry)
    }

    /// Retrieves a memory entry by its unique identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: The retrieved `MemoryEntry`
    /// - `Err`: A `CustomError` if the memory is not found or retrieval fails
    pub async fn get_memory_by_id(&self, id: Uuid) -> Result<MemoryEntry, CustomError> {
        let memories = self.vector_repo.get_memories_by_ids(&[id]).await?;
        Ok(memories.into_iter().next().unwrap())
    }

    /// Retrieves multiple memory entries by their unique identifiers.
    ///
    /// # Parameters
    ///
    /// - `ids`: A slice of UUIDs of the memory entries to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the retrieved memories
    /// - `Err`: A `CustomError` if retrieval fails
    pub async fn get_memories_by_ids(&self, ids: &[Uuid]) -> Result<Vec<MemoryEntry>, CustomError> {
        self.vector_repo.get_memories_by_ids(ids).await
    }

    /// Updates an existing memory entry.
    ///
    /// # Parameters
    ///
    /// - `input`: A `MemoryInput` containing the updated data and ID of the entry to update
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `MemoryEntry` containing the updated memory
    /// - `Err`: A `CustomError` if:
    ///     - The input does not contain an ID
    ///     - Embedding generation fails
    ///     - Memory validation fails
    ///     - The update operation fails
    pub async fn update_memory(&self, metadata: VectorMetadata) -> Result<MemoryEntry, CustomError> {
        let embedding = self.embed_repo.generate_embedding(&metadata.content)?;
        let entry = MemoryEntry::new(metadata, embedding)?;
        self.vector_repo.update_memory(&entry).await?;

        Ok(entry)
    }

    /// Deletes a memory entry identified by the given UUID.
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
    /// - `Err`: A `CustomError` if the deletion fails
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.vector_repo.delete_memory(id).await
    }

    /// Searches for memory entries that are semantically similar to the given query.
    ///
    /// # Description
    ///
    /// Generates an embedding for the query and delegates the search to the vector repository.
    ///
    /// # Parameters
    ///
    /// - `query`: The search query string to find similar memories
    /// - `top_k`: The maximum number of results to return
    /// - `filters`: Optional filters to apply to the search results
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the search results
    /// - `Err`: A `CustomError` if embedding generation or search fails
    pub async fn search_memories(&self, query: &str, top_k: usize, filters: Option<MemoryFilters>) -> Result<Vec<MemoryEntry>, CustomError> {
        let query_embedding = self.embed_repo.generate_embedding(query)?;
        self.vector_repo.search_memory(&query_embedding, top_k, filters.as_ref()).await
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// # Description
    ///
    /// Generates embeddings for all inputs in bulk, constructs MemoryEntry instances,
    /// and persists all entries in a single operation via the vector repository.
    ///
    /// # Parameters
    ///
    /// - `inputs`: A slice of `MemoryInput` containing the data for each memory entry
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the created memories
    /// - `Err`: A `CustomError` if embedding generation, memory validation, or bulk storage fails
    pub async fn bulk_create_memories(&self, metadata_list: &[VectorMetadata]) -> Result<Vec<MemoryEntry>, CustomError> {
        // Extract content strings for bulk embedding generation
        let contents: Vec<&str> = metadata_list.iter().map(|metadata| metadata.content.as_str()).collect();

        // Generate embeddings in bulk
        let embeddings = self.embed_repo.bulk_generate_embeddings(&contents)?;

        // Create memory entries from metadata and embeddings
        let mut entries = Vec::with_capacity(metadata_list.len());
        let metadata_embedding_pairs = metadata_list.iter().zip(embeddings.into_iter());
        for (metadata, embedding) in metadata_embedding_pairs {
            let entry = MemoryEntry::new(metadata.clone(), embedding)?;
            entries.push(entry);
        }

        // Store all entries in bulk
        self.vector_repo.bulk_insert(&entries).await?;

        Ok(entries)
    }
}
