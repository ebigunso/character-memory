use crate::models::memory::dto::ScoredMemory;
use crate::models::memory::MemoryEntry;

#[derive(Debug, Clone)]
pub struct ScoredMemoryEntry {
    pub entry: MemoryEntry,
    pub score: f32,
}

impl ScoredMemoryEntry {
    pub fn into_public(self) -> ScoredMemory {
        ScoredMemory {
            memory: self.entry.into_public(),
            score: self.score,
        }
    }
}
