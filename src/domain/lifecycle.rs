use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ObjectType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LifecycleDtoValidationError {
    #[error("rationale must not be empty")]
    EmptyRationale,
    #[error("correction origin provenance is required")]
    EmptyCorrectionOrigin,
    #[error("replacement derived memory text must not be empty")]
    EmptyReplacementText,
    #[error(
        "replacement derived memory must reference at least one source episode or observation"
    )]
    MissingReplacementSource,
    #[error("correction requires at least one target")]
    MissingCorrectionTarget,
    #[error("forget requires at least one target")]
    MissingForgetTarget,
    #[error("unsupported lifecycle target: {0:?}")]
    UnsupportedLifecycleTarget(ObjectType),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePolicyKnob {
    CorrectionRetainOriginalSourceObjects,
    CorrectionRequireOriginalSourceMatch,
    CorrectionCascadeToThreads,
    ForgetPreserveOriginalRawRefs,
    ForgetArchivePreserveOriginalRawRefs,
}
