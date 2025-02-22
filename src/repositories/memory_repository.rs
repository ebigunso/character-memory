use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::internal::MemoryEntry;
use crate::models::public::{MemoryFilters, MemoryInput};
use crate::repositories::{EmbeddingRepository, VectorMemoryRepository};

/// Provides high-level operations for managing memory entries.
///
/// Delegates embedding generation to an EmbeddingRepository and storage/retrieval to a
/// VectorMemoryRepository. All dependencies are injected as arguments.
pub(crate) struct MemoryRepository {
    pub embed_repo: Box<dyn EmbeddingRepository>,
    pub vector_repo: Box<dyn VectorMemoryRepository>,
}

impl MemoryRepository
{
    /// Creates a new MemoryRepository instance.
    ///
    /// # Arguments
    /// * `embed_repo` - The embedding repository to generate embeddings.
    /// * `vector_repo` - The vector memory repository for storing memory entries.
    pub fn new(embed_repo: Box<dyn EmbeddingRepository>, vector_repo: Box<dyn VectorMemoryRepository>) -> Self {
        Self { embed_repo, vector_repo }
    }

    /// Creates a new memory entry.
    ///
    /// Generates an embedding for the input content, validates and constructs a MemoryEntry,
    /// and persists the memory entry via the vector repository.
    ///
    /// # Errors
    /// Returns a CustomError if embedding generation, memory validation, or storage fails.
    pub async fn create_memory(&self, input: MemoryInput) -> Result<MemoryEntry, CustomError> {
        let embedding = self.embed_repo.generate_embedding(&input.content)?;
        let entry = MemoryEntry::new(input, embedding)?;
        self.vector_repo.store_memory(&entry).await?;

        Ok(entry)
    }

    /// Retrieves a memory entry by its unique identifier.
    ///
    /// Unimplemented because the underlying vector repository does not support direct lookup.
    ///
    /// # Errors
    /// Always returns an error as this method is not implemented.
    pub async fn get_memory_by_id(&self, _id: Uuid) -> Result<MemoryEntry, CustomError> {
        unimplemented!("get_memory_by_id is not implemented in VectorMemoryRepository")
    }

    /// Updates an existing memory entry.
    ///
    /// # Arguments
    /// * `input` - A MemoryInput struct containing the updated data and the ID of the entry to update
    ///
    /// # Errors
    /// Returns a CustomError if:
    /// * The input does not contain an ID
    /// * Embedding generation fails
    /// * Memory validation fails
    /// * The update operation fails
    pub async fn update_memory(&self, input: MemoryInput) -> Result<MemoryEntry, CustomError> {
        let _id = input.id.ok_or_else(|| CustomError::MemoryValidation("ID is required for update operation".to_string()))?;

        let embedding = self.embed_repo.generate_embedding(&input.content)?;
        let entry = MemoryEntry::new(input, embedding)?;
        self.vector_repo.update_memory(&entry).await?;

        Ok(entry)
    }

    /// Deletes a memory entry identified by the given UUID.
    ///
    /// # Errors
    /// Returns a CustomError if the deletion fails.
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.vector_repo.delete_memory(id).await
    }

    /// Searches for memory entries that are semantically similar to the given query.
    ///
    /// Generates an embedding for the query and delegates the search to the vector repository.
    ///
    /// # Errors
    /// Returns a CustomError if embedding generation or search fails.
    pub async fn search_memories(&self, query: &str, top_k: usize, filters: Option<MemoryFilters>) -> Result<Vec<MemoryEntry>, CustomError> {
        let query_embedding = self.embed_repo.generate_embedding(query)?;
        self.vector_repo.search_memory(&query_embedding, top_k, filters.as_ref()).await
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// Generates embeddings for all inputs in bulk, constructs MemoryEntry instances,
    /// and persists all entries in a single operation via the vector repository.
    ///
    /// # Errors
    /// Returns a CustomError if embedding generation, memory validation, or bulk storage fails.
    pub async fn bulk_create_memories(&self, inputs: &[MemoryInput]) -> Result<Vec<MemoryEntry>, CustomError> {
        // Extract content strings for bulk embedding generation
        let contents: Vec<&str> = inputs.iter().map(|input| input.content.as_str()).collect();

        // Generate embeddings in bulk
        let embeddings = self.embed_repo.bulk_generate_embeddings(&contents)?;

        // Create memory entries from inputs and embeddings
        let mut entries = Vec::with_capacity(inputs.len());
        let input_embedding_pairs = inputs.iter().zip(embeddings.into_iter());
        for (input, embedding) in input_embedding_pairs {
            let entry = MemoryEntry::new(input.clone(), embedding)?;
            entries.push(entry);
        }

        // Store all entries in bulk
        self.vector_repo.bulk_insert(&entries).await?;

        Ok(entries)
    }
}
