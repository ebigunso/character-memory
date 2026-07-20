use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCandidateKind {
    Episode,
    Observation,
    Entity,
    MemoryThread,
    DerivedMemory,
    MemoryLink,
    VectorIndex,
    StatsUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateValidation {
    pub candidate_index: usize,
    pub candidate_kind: MemoryCandidateKind,
    pub status: CandidateValidationStatus,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CandidateValidation {
    pub fn valid(candidate_index: usize, candidate_kind: MemoryCandidateKind) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Valid,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(
        candidate_index: usize,
        candidate_kind: MemoryCandidateKind,
        error: impl Into<String>,
    ) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Invalid,
            errors: vec![error.into()],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateValidationStatus {
    Valid,
    Invalid,
}
