// Module declarations
pub(crate) mod external_services {
    pub(crate) mod openai_embedding_repository;
    pub(crate) mod qdrant_vector_memory_repository;

    pub(crate) use openai_embedding_repository::OpenAIEmbeddingRepository;
    pub(crate) use qdrant_vector_memory_repository::QdrantVectorMemoryRepository;
}
