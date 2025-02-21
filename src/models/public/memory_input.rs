use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInput {
    pub content: String,
    pub memory_type: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub location_text: Option<String>,
    pub participants: Option<Vec<String>>,
}
