// Module declarations
pub(crate) mod external_services {
    mod openai_embedding_repository;
    mod qdrant_vector_memory_repository;

    pub(crate) use openai_embedding_repository::OpenAIEmbeddingRepository;
    pub(crate) use qdrant_vector_memory_repository::QdrantVectorMemoryRepository;
}
