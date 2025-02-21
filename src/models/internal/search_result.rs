use serde_json::{Value, Map};

// Domain-specific representation of a vector database search result.
#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub id: String,
    pub payload: Map<String, Value>,
    pub vector: Vec<f32>,
    pub score: f32,
}
