pub(crate) mod openai;
pub(crate) mod oxigraph;
pub(crate) mod qdrant;
pub(crate) mod stats;

pub(crate) use openai::OpenAIEmbeddingProvider;
pub(crate) use qdrant::QdrantVectorCandidateStore;
// Stats adapters are selected by composition/tests in different target sets; remove when target use converges.
#[allow(unused_imports)]
pub(crate) use stats::{InMemoryRetrievalStatsStore, SqliteRetrievalStatsStore};

#[cfg(test)]
// Unit tests share this adapter through the barrel selectively; remove when tests import stats:: directly.
#[allow(unused_imports)]
pub(crate) use stats::noop_retrieval_stats_store;
