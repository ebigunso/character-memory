use thiserror::Error;

use super::ObjectType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceReferenceKind {
    Raw,
    SettingKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LifecycleDtoValidationError {
    #[error("rationale must not be empty")]
    EmptyRationale,
    #[error("correction origin provenance is required")]
    EmptyCorrectionOrigin,
    #[error("replacement derived memory text must not be empty")]
    EmptyReplacementText,
    #[error(
        "replacement derived memory must cite a source episode or observation, or declare application-given grounding"
    )]
    MissingReplacementSource,
    #[error("correcting a given belief without source experiences requires an explicit replacement marked given_by_application")]
    MissingGivenReplacement,
    #[error(transparent)]
    InvalidBelief(#[from] super::BeliefValidationError),
    #[error("correction requires at least one target")]
    MissingCorrectionTarget,
    #[error("forget requires at least one target")]
    MissingForgetTarget,
    #[error("unsupported lifecycle target: {0:?}")]
    UnsupportedLifecycleTarget(ObjectType),
}
