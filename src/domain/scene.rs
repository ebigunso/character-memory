use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use super::MemoryId;

/// The situation as perceived. Omitted participants do not mean nobody was present.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scene {
    pub time: DateTime<FixedOffset>,
    pub participants: Vec<SceneParticipant>,
    pub setting: SceneSetting,
    pub custom_values: BTreeMap<String, String>,
}

impl Scene {
    pub fn at(time: DateTime<FixedOffset>) -> Self {
        Self {
            time,
            participants: Vec::new(),
            setting: SceneSetting::default(),
            custom_values: BTreeMap::new(),
        }
    }

    pub fn now() -> Self {
        Self::at(Utc::now().fixed_offset())
    }

    pub(crate) fn without_blank_participants(mut self) -> Self {
        self.participants.retain(|participant| {
            participant.key.is_some()
                || [&participant.name, &participant.description]
                    .into_iter()
                    .flatten()
                    .any(|words| !words.trim().is_empty())
        });
        self
    }

    pub(crate) fn participant_keys(&self) -> impl Iterator<Item = MemoryId> + '_ {
        self.participants
            .iter()
            .filter_map(|participant| participant.key)
    }
}

/// Perceived words and a known identity can coexist; blank entries are omitted.
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

/// Internal context values, never identifiers supplied to retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScopeKey {
    Setting(String),
    Custom { name: String, value: String },
}

impl Scene {
    pub(crate) fn scope_keys(&self) -> Vec<ScopeKey> {
        self.setting
            .key
            .iter()
            .cloned()
            .map(ScopeKey::Setting)
            .chain(
                self.custom_values
                    .iter()
                    .map(|(name, value)| ScopeKey::Custom {
                        name: name.clone(),
                        value: value.clone(),
                    }),
            )
            .collect()
    }
}
