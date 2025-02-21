use serde_json::Value;

// Domain-specific representation of a vector database search query.
#[derive(Debug, Clone)]
pub(crate) struct SearchQuery {
    pub collection_name: String,
    pub vector: Vec<f32>,
    pub limit: u64,
    pub filter: Option<Value>,
    pub with_payload: bool,
}
