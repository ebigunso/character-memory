use serde::{Deserialize, Serialize};

use super::MemoryObjectRef;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphExpansionBoundedReason {
    NodeLimit,
    Timeout,
    HubLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExpansionBoundedFailureTrace {
    pub reason: GraphExpansionBoundedReason,
    pub at: Option<MemoryObjectRef>,
}
