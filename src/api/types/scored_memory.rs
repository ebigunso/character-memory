use serde::{Deserialize, Serialize};

use super::Memory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}
