use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use super::MemoryId;

/// A commitment the character holds about one of a memory's notion subjects.
/// Someone else's claim or the character's doubt remains text without an assertion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeliefAssertion {
    pub subject: MemoryId,
    #[serde(flatten)]
    pub predicate: BeliefPredicate,
}

/// Closed vocabulary of assertions that recall can read mechanically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "predicate", rename_all = "snake_case")]
pub enum BeliefPredicate {
    KnownAs { name: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BeliefValidationError {
    #[error("a belief given by the application cannot also cite source experiences")]
    GivenWithSources,
    #[error("a belief given by the application must have a notion subject")]
    GivenWithoutSubject,
    #[error("assertion subject {subject} must be among the memory's notion subjects")]
    AssertionSubjectNotInMemory { subject: MemoryId },
    #[error("asserted name for {subject} must not be empty after normalization")]
    EmptyAssertionName { subject: MemoryId },
}

pub(crate) fn validate_belief(
    subjects: &[MemoryId],
    has_sources: bool,
    given_by_application: bool,
    assertions: &[BeliefAssertion],
) -> Result<(), BeliefValidationError> {
    if given_by_application {
        if has_sources {
            return Err(BeliefValidationError::GivenWithSources);
        }
        if subjects.is_empty() {
            return Err(BeliefValidationError::GivenWithoutSubject);
        }
    }
    for assertion in assertions {
        if !subjects.contains(&assertion.subject) {
            return Err(BeliefValidationError::AssertionSubjectNotInMemory {
                subject: assertion.subject,
            });
        }
        match &assertion.predicate {
            BeliefPredicate::KnownAs { name } if normalize_name(name).is_empty() => {
                return Err(BeliefValidationError::EmptyAssertionName {
                    subject: assertion.subject,
                });
            }
            BeliefPredicate::KnownAs { .. } => {}
        }
    }
    Ok(())
}

pub(crate) fn normalize_name(name: &str) -> String {
    // ponytail: lowercase does not equate sharp-s with ss; use full case folding
    // when an observed multilingual name miss warrants that broader equivalence.
    name.nfkc()
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
