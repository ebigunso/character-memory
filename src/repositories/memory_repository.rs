use uuid::Uuid;

use crate::api::types::MemoryFilters;
use crate::errors::CustomError;
use crate::models::memory::{MemoryEntry, ScoredMemoryEntry};
use crate::models::vector::VectorMetadata;
use crate::repositories::VectorMemoryRepository;
use crate::EmbeddingRepository;

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
    pub fn new(
        embed_repo: Box<dyn EmbeddingRepository>,
        vector_repo: Box<dyn VectorMemoryRepository>,
    ) -> Self {
        Self {
            embed_repo,
            vector_repo,
        }
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
    /// Generates an embedding for the memory content, validates and constructs a MemoryEntry,
    /// and persists the memory entry via the vector repository.
    ///
    /// # Parameters
    ///
    /// - `metadata`: `VectorMetadata` containing the data for the new memory entry
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `MemoryEntry` containing the created memory
    /// - `Err`: A `CustomError` if embedding generation, memory validation, or storage fails
    pub async fn create_memory(
        &self,
        metadata: VectorMetadata,
    ) -> Result<MemoryEntry, CustomError> {
        let embedding = self
            .embed_repo
            .generate_embedding(&metadata.content)
            .await?;
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
        memories
            .into_iter()
            .next()
            .ok_or_else(|| CustomError::DatabaseError(format!("Memory with ID {id} not found")))
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
    /// - `metadata`: `VectorMetadata` containing the updated memory data, including the entry's ID
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `MemoryEntry` containing the updated memory
    /// - `Err`: A `CustomError` if:
    ///     - The metadata does not contain an ID
    ///     - Embedding generation fails
    ///     - Memory validation fails
    ///     - The update operation fails
    pub async fn update_memory(
        &self,
        metadata: VectorMetadata,
    ) -> Result<MemoryEntry, CustomError> {
        let embedding = self
            .embed_repo
            .generate_embedding(&metadata.content)
            .await?;
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
    pub async fn search_memories(
        &self,
        query: &str,
        top_k: usize,
        filters: Option<MemoryFilters>,
    ) -> Result<Vec<ScoredMemoryEntry>, CustomError> {
        let query_embedding = self.embed_repo.generate_embedding(query).await?;
        self.vector_repo
            .search_memory(&query_embedding, top_k, filters.as_ref())
            .await
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// # Description
    ///
    /// Generates embeddings for all metadata entries in bulk, constructs MemoryEntry instances,
    /// and persists all entries in a single operation via the vector repository.
    ///
    /// # Parameters
    ///
    /// - `metadata_list`: A slice of `VectorMetadata` describing each memory entry to create
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `MemoryEntry` containing the created memories
    /// - `Err`: A `CustomError` if embedding generation, memory validation, or bulk storage fails
    pub async fn bulk_create_memories(
        &self,
        metadata_list: &[VectorMetadata],
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        // Extract content strings for bulk embedding generation
        let contents: Vec<&str> = metadata_list
            .iter()
            .map(|metadata| metadata.content.as_str())
            .collect();

        // Generate embeddings in bulk
        let embeddings = self.embed_repo.bulk_generate_embeddings(&contents).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct CapturingVectorRepo {
        last_stored_embedding: Arc<Mutex<Option<Vec<f32>>>>,
        last_query_vector: Arc<Mutex<Option<Vec<f32>>>>,
    }

    impl CapturingVectorRepo {
        fn take_last_stored_embedding(&self) -> Option<Vec<f32>> {
            self.last_stored_embedding.lock().expect("lock").take()
        }

        fn take_last_query_vector(&self) -> Option<Vec<f32>> {
            self.last_query_vector.lock().expect("lock").take()
        }
    }

    #[async_trait]
    impl VectorMemoryRepository for CapturingVectorRepo {
        async fn init_collection(&self) -> Result<(), CustomError> {
            Ok(())
        }

        async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
            *self.last_stored_embedding.lock().expect("lock") = Some(memory.embedding.clone());
            Ok(())
        }

        async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
            self.store_memory(memory).await
        }

        async fn delete_memory(&self, _id: Uuid) -> Result<(), CustomError> {
            Ok(())
        }

        async fn search_memory<'a>(
            &'a self,
            query_vector: &'a [f32],
            _top_k: usize,
            _filters: Option<&'a MemoryFilters>,
        ) -> Result<Vec<ScoredMemoryEntry>, CustomError> {
            *self.last_query_vector.lock().expect("lock") = Some(query_vector.to_vec());
            Ok(Vec::new())
        }

        async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
            let last = memories
                .last()
                .ok_or_else(|| CustomError::DatabaseError("No memories provided".to_string()))?;
            self.store_memory(last).await
        }

        async fn get_memories_by_ids<'a>(
            &'a self,
            _ids: &'a [Uuid],
        ) -> Result<Vec<MemoryEntry>, CustomError> {
            Ok(Vec::new())
        }
    }

    struct FakeEmbeddingRepo;

    impl FakeEmbeddingRepo {
        fn embed(text: &str) -> Vec<f32> {
            vec![text.len() as f32, 1.0, 0.0]
        }
    }

    #[async_trait]
    impl EmbeddingRepository for FakeEmbeddingRepo {
        async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
            Ok(Self::embed(text))
        }

        async fn bulk_generate_embeddings<'a>(
            &self,
            texts: &'a [&'a str],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(texts.iter().map(|t| Self::embed(t)).collect())
        }
    }

    #[tokio::test]
    async fn create_memory_uses_injected_embedder() {
        let vector_repo = CapturingVectorRepo::default();
        let vector_repo_handle = vector_repo.clone();
        let repo = MemoryRepository::new(Box::new(FakeEmbeddingRepo), Box::new(vector_repo));

        let id = Uuid::new_v4();
        let metadata = VectorMetadata::new_semantic(id, "hello".to_string());

        let created = repo.create_memory(metadata).await.expect("create succeeds");
        assert_eq!(created.id, id);

        let stored_embedding = vector_repo_handle
            .take_last_stored_embedding()
            .expect("expected stored embedding");
        assert_eq!(stored_embedding, FakeEmbeddingRepo::embed("hello"));
    }

    #[tokio::test]
    async fn search_memories_uses_injected_embedder_for_query() {
        let vector_repo = CapturingVectorRepo::default();
        let vector_repo_handle = vector_repo.clone();
        let repo = MemoryRepository::new(Box::new(FakeEmbeddingRepo), Box::new(vector_repo));

        repo.search_memories("query", 5, None)
            .await
            .expect("search succeeds");

        let query_vector = vector_repo_handle
            .take_last_query_vector()
            .expect("expected captured query vector");
        assert_eq!(query_vector, FakeEmbeddingRepo::embed("query"));
    }

    #[tokio::test]
    async fn bulk_create_memories_uses_bulk_embedding() {
        let vector_repo = CapturingVectorRepo::default();
        let vector_repo_handle = vector_repo.clone();
        let repo = MemoryRepository::new(Box::new(FakeEmbeddingRepo), Box::new(vector_repo));

        let a = VectorMetadata::new_semantic(Uuid::new_v4(), "a".to_string());
        let b = VectorMetadata::new_semantic(Uuid::new_v4(), "bbbb".to_string());
        let _ = repo
            .bulk_create_memories(&[a, b])
            .await
            .expect("bulk create succeeds");

        let stored_embedding = vector_repo_handle
            .take_last_stored_embedding()
            .expect("expected stored embedding");
        assert_eq!(stored_embedding, FakeEmbeddingRepo::embed("bbbb"));
    }
}
