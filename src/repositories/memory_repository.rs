use crate::models::public::{MemoryInput, MemoryFilters};
use crate::models::internal::MemoryEntry;
use crate::errors::custom::CustomError;
use crate::repositories::embedding_repository::EmbeddingRepository;
use crate::repositories::vector_memory_repository::VectorMemoryRepository;
use crate::databases::vector_database::VectorDatabase;
use uuid::Uuid;

/// MemoryRepository provides high‑level operations for managing memory entries.
/// It delegates embedding generation to an EmbeddingRepository and storage/retrieval
/// to a VectorMemoryRepository. All dependencies are injected as arguments.
pub struct MemoryRepository<T: VectorDatabase> {
    pub embed_repo: EmbeddingRepository,
    pub vector_repo: VectorMemoryRepository<T>,
}

impl<T: VectorDatabase> MemoryRepository<T> {
    /// Constructs a new MemoryRepository with the provided embedding and vector memory repositories.
    pub fn new(embed_repo: EmbeddingRepository, vector_repo: VectorMemoryRepository<T>) -> Self {
        Self { embed_repo, vector_repo }
    }

    /// Creates a new memory entry.
    /// Steps:
    /// 1. Generates an embedding for the input content using the injected EmbeddingRepository.
    /// 2. Validates and constructs a MemoryEntry.
    /// 3. Persists the memory entry via the injected VectorMemoryRepository.
    pub async fn create_memory(&self, input: MemoryInput) -> Result<MemoryEntry, CustomError> {
        let embedding = self.embed_repo.generate_embedding(&input.content)?;
        let entry = MemoryEntry::new(input, embedding)?;
        self.vector_repo.store_memory(&entry).await?;
        Ok(entry)
    }

    /// Retrieves a memory entry by its unique identifier.
    /// Currently unimplemented because the underlying vector repository does not support direct lookup.
    pub async fn get_memory_by_id(&self, _id: Uuid) -> Result<MemoryEntry, CustomError> {
        unimplemented!("get_memory_by_id is not implemented in VectorMemoryRepository")
    }

    /// Updates an existing memory entry.
    /// Unimplemented due to ambiguity in MemoryInput (e.g., lack of an ID).
    pub async fn update_memory(&self, _input: MemoryInput) -> Result<MemoryEntry, CustomError> {
        unimplemented!("Update memory is not implemented because MemoryInput does not include an ID")
    }

    /// Deletes a memory entry identified by the given UUID.
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.vector_repo.delete_memory(id).await
    }

    /// Searches for memory entries that are semantically similar to the query.
    /// It generates an embedding for the query and delegates the search to the vector repository.
    pub async fn search_memories(&self, query: &str, top_k: usize, filters: Option<MemoryFilters>) -> Result<Vec<MemoryEntry>, CustomError> {
        let query_embedding = self.embed_repo.generate_embedding(query)?;
        self.vector_repo.search_memory(&query_embedding, top_k, filters.as_ref()).await
    }

    /// Processes multiple memory inputs in a batch operation.
    pub async fn bulk_create_memories(&self, inputs: &[MemoryInput]) -> Result<Vec<MemoryEntry>, CustomError> {
        let mut entries = Vec::new();
        for input in inputs {
            let embedding = self.embed_repo.generate_embedding(&input.content)?;
            let entry = MemoryEntry::new(input.clone(), embedding)?;
            self.vector_repo.store_memory(&entry).await?;
            entries.push(entry);
        }
        Ok(entries)
    }
}
