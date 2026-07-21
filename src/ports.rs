// Port traits and their query/result value types. Adapters implement these
// contracts; use-case pipelines depend only on the traits.
pub(crate) mod embedder;
pub(crate) mod graph_authority;
pub(crate) mod retrieval_stats;
pub(crate) mod vector_candidate;
