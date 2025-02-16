use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInput {
    pub content: String,
    pub memory_type: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub location_text: Option<String>,
    pub participants: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFilters {
    pub memory_type: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub location_text: Option<String>,
    pub participants: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub content: String,
    pub memory_type: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub location_text: Option<String>,
    pub participants: Option<Vec<String>>,
}
