use serde_json::{Value, Map};

// Domain-specific representation of a vector database point.
// This type abstracts the data stored in a vector database entry.
#[derive(Debug, Clone)]
pub(crate) struct Point {
    pub id: Option<String>,
    pub payload: Map<String, Value>,
    pub vector: Vec<f32>,
}
