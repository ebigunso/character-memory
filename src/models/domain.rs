use serde_json::{Value, Map};

// Domain-specific representation of a vector database point.
// This type abstracts the data stored in a vector database entry.
#[derive(Debug, Clone)]
pub(crate) struct Point {
    pub id: Option<String>,
    pub payload: Map<String, Value>,
    pub vector: Vec<f32>,
}

// Domain-specific representation of a vector database search query.
#[derive(Debug, Clone)]
pub(crate) struct SearchQuery {
    pub collection_name: String,
    pub vector: Vec<f32>,
    pub limit: u64,
    pub filter: Option<Value>,
    pub with_payload: bool,
}

// Domain-specific representation of a vector database search result.
#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub id: String,
    pub payload: Map<String, Value>,
    pub vector: Vec<f32>,
    pub score: f32,
}
