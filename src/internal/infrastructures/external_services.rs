mod openai_embedding_provider;
mod qdrant_payload;
mod qdrant_vector_candidate_store;
mod qdrant_vector_memory_repository;

pub(crate) use openai_embedding_provider::OpenAIEmbeddingProvider;
// Transitional v0.1 adapter surface: exported for downstream storage pipeline
// chunks before the concrete retrieval path consumes it directly.
#[allow(unused_imports)]
pub(crate) use qdrant_payload::{qdrant_payload_index_fields, qdrant_payload_map};
#[allow(unused_imports)]
pub(crate) use qdrant_vector_candidate_store::QdrantVectorCandidateStore;
pub(crate) use qdrant_vector_memory_repository::QdrantVectorMemoryRepository;
