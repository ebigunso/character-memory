use crate::api::types::ScoredMemory;
use crate::models::memory::MemoryEntry;

#[derive(Debug, Clone)]
pub(crate) struct ScoredMemoryEntry {
    pub(crate) entry: MemoryEntry,
    pub(crate) score: f32,
}

impl ScoredMemoryEntry {
    pub(crate) fn into_public(self) -> ScoredMemory {
        ScoredMemory {
            memory: self.entry.into_public(),
            score: self.score,
        }
    }
}
