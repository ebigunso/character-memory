use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::MemoryObjectRef;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphFailureMode {
    #[default]
    AllowPartialResults,
    FailClosed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphExpansionBoundedReason {
    NodeLimit,
    Timeout,
    HubLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[error("{reason:?} at {at:?}")]
pub struct GraphExpansionBoundedFailureTrace {
    pub reason: GraphExpansionBoundedReason,
    pub at: Option<MemoryObjectRef>,
}
