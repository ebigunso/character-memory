use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub content: String,
    pub memory_type: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub location_text: Option<String>,
    pub participants: Option<Vec<String>>,
}
