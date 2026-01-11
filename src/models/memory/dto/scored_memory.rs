use serde::{Deserialize, Serialize};

use crate::models::memory::dto::Memory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}
