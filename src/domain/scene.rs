use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::MemoryId;

/// The situation as perceived. Omitted participants do not mean nobody was present.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scene {
    pub time: DateTime<Utc>,
    pub participants: Vec<SceneParticipant>,
    pub setting: SceneSetting,
    pub custom_values: BTreeMap<String, String>,
    /// Thread or open-loop context; writes reject it.
    pub activity: Option<SceneActivity>,
}

impl Scene {
    pub fn at(time: DateTime<Utc>) -> Self {
        Self {
            time,
            participants: Vec::new(),
            setting: SceneSetting::default(),
            custom_values: BTreeMap::new(),
            activity: None,
        }
    }

    pub fn now() -> Self {
        Self::at(Utc::now())
    }

    pub(crate) fn participant_keys(&self) -> impl Iterator<Item = MemoryId> + '_ {
        self.participants
            .iter()
            .filter_map(|participant| participant.key)
    }
}

/// Perceived words and a known identity can coexist; writes require at least one field.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneParticipant {
    pub key: Option<MemoryId>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneSetting {
    /// An application-owned context key, not a notion identity.
    pub key: Option<String>,
    pub words: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum SceneActivity {
    Thread(MemoryId),
    OpenLoop(MemoryId),
}
