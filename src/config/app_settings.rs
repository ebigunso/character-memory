use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};

use crate::domain::{ObjectType, RelationType};
use crate::errors::{ConfigValidationError, ConfigValidationReason, CustomError};
use crate::models::vector::EmbeddingModel;

#[derive(Debug, Deserialize)]
pub struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_path: SecretString,
    openai_api_key: SecretString,
    embedding_model: SecretString,
    #[serde(default)]
    graph_store_mode: GraphStoreMode,
    #[serde(default)]
    retrieval_stats_store_mode: RetrievalStatsStoreMode,
    #[serde(default = "default_retrieval_stats_path")]
    retrieval_stats_path: PathBuf,
    #[serde(default)]
    retrieval_stats_health_fail_mode: RetrievalStatsHealthFailMode,
    #[serde(default = "default_selectivity_smoothing_alpha")]
    selectivity_smoothing_alpha: f64,
    #[serde(default = "default_selectivity_gamma")]
    selectivity_gamma: f64,
    #[serde(default)]
    retrieval: RetrievalSettings,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct RetrievalSettings {
    #[serde(default)]
    fanout: RetrievalFanoutSettings,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct RetrievalFanoutSettings {
    #[serde(default)]
    about_entity: FanoutObjectSettings,
    #[serde(default)]
    participant_entity: FanoutObjectSettings,
    #[serde(default)]
    part_of_thread: FanoutObjectSettings,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct FanoutObjectSettings {
    derived_memory: Option<FanoutBudgetSettings>,
    episode: Option<FanoutBudgetSettings>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub(crate) struct FanoutBudgetSettings {
    min: usize,
    max: usize,
}

impl FanoutBudgetSettings {
    pub(crate) fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }

    pub(crate) fn min(self) -> usize {
        self.min
    }

    pub(crate) fn max(self) -> usize {
        self.max
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GraphStoreMode {
    #[default]
    Persistent,
    InMemory,
}

impl GraphStoreMode {
    fn parse(value: &str) -> Result<Self, CustomError> {
        match value {
            "persistent" => Ok(Self::Persistent),
            "in_memory" => Ok(Self::InMemory),
            other => Err(ConfigValidationError {
                keys: vec!["GRAPH_STORE_MODE"],
                reason: ConfigValidationReason::OutOfDomain {
                    expected: "persistent or in_memory",
                    actual: other.to_owned(),
                },
            }
            .into()),
        }
    }
}

impl<'de> Deserialize<'de> for GraphStoreMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStatsStoreMode {
    #[default]
    Sqlite,
    InMemory,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStatsHealthFailMode {
    #[default]
    Conservative,
}

impl Settings {
    /// Creates a new Settings instance.
    ///
    /// # Description
    ///
    /// Primary constructor for creating a Settings instance. Takes a pre-configured Config object that defines all required settings.
    /// This allows for flexible configuration sourcing while maintaining a clean initialization interface.
    ///
    /// # Parameters
    ///
    /// - `config`: A `config::Config` instance containing all required settings:
    ///     - `qdrant_connection_string`: Connection string for Qdrant database
    ///     - `oxigraph_path`: Local filesystem path for the Oxigraph database
    ///     - `openai_api_key`: API key for OpenAI services
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with the provided configuration
    /// - `Err`: A `CustomError` if any required settings are missing or invalid
    pub fn new(config: Config) -> Result<Self, CustomError> {
        let settings: Self = config.try_deserialize().map_err(|e| {
            CustomError::ConfigParseError(format!("Failed to parse external configuration: {e}"))
        })?;
        settings.validate_selectivity_settings()?;
        settings.validate_fanout_settings()?;
        Ok(settings)
    }

    pub fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    pub fn get_graph_store_mode(&self) -> GraphStoreMode {
        self.graph_store_mode
    }

    pub fn get_retrieval_stats_store_mode(&self) -> RetrievalStatsStoreMode {
        self.retrieval_stats_store_mode
    }

    pub fn get_retrieval_stats_path(&self) -> &Path {
        &self.retrieval_stats_path
    }

    pub fn get_retrieval_stats_health_fail_mode(&self) -> RetrievalStatsHealthFailMode {
        self.retrieval_stats_health_fail_mode
    }

    pub fn get_selectivity_smoothing_alpha(&self) -> f64 {
        self.selectivity_smoothing_alpha
    }

    pub fn get_selectivity_gamma(&self) -> f64 {
        self.selectivity_gamma
    }

    pub(crate) fn get_retrieval_fanout_budgets(
        &self,
    ) -> Vec<(RelationType, ObjectType, FanoutBudgetSettings)> {
        self.retrieval.fanout.budgets()
    }

    pub fn get_oxigraph_path(&self) -> Result<PathBuf, CustomError> {
        let configured_path = self.oxigraph_path.expose_secret();
        if configured_path.contains("://") {
            return Err(ConfigValidationError {
                keys: vec!["OXIGRAPH_PATH"],
                reason: ConfigValidationReason::OutOfDomain {
                    expected: "a local filesystem path",
                    actual: configured_path.to_owned(),
                },
            }
            .into());
        }

        let path = Path::new(configured_path);
        if path.as_os_str().is_empty() {
            return Err(ConfigValidationError {
                keys: vec!["OXIGRAPH_PATH"],
                reason: ConfigValidationReason::MissingValue,
            }
            .into());
        }
        Ok(path.to_path_buf())
    }

    pub fn get_openai_api_key(&self) -> &str {
        self.openai_api_key.expose_secret()
    }

    pub fn get_embedding_vector_size(&self) -> Result<usize, CustomError> {
        Ok(self.get_embedding_model()?.vector_size() as usize)
    }

    pub(crate) fn get_embedding_model(&self) -> Result<EmbeddingModel, CustomError> {
        self.embedding_model.expose_secret().parse()
    }

    fn validate_selectivity_settings(&self) -> Result<(), CustomError> {
        validate_positive_f64(
            "selectivity_smoothing_alpha",
            self.selectivity_smoothing_alpha,
        )?;
        validate_positive_f64("selectivity_gamma", self.selectivity_gamma)
    }

    fn validate_fanout_settings(&self) -> Result<(), CustomError> {
        self.retrieval.fanout.validate_supported_targets()?;
        for (relation, object_type, budget) in self.get_retrieval_fanout_budgets() {
            validate_fanout_budget(fanout_budget_name(relation, object_type), budget)?;
        }
        Ok(())
    }
}

impl RetrievalFanoutSettings {
    fn validate_supported_targets(&self) -> Result<(), CustomError> {
        reject_unsupported_fanout_target(
            "retrieval.fanout.about_entity.episode",
            self.about_entity.episode,
        )?;
        reject_unsupported_fanout_target(
            "retrieval.fanout.participant_entity.derived_memory",
            self.participant_entity.derived_memory,
        )?;
        reject_unsupported_fanout_target(
            "retrieval.fanout.part_of_thread.episode",
            self.part_of_thread.episode,
        )?;
        Ok(())
    }

    fn budgets(&self) -> Vec<(RelationType, ObjectType, FanoutBudgetSettings)> {
        let mut budgets = default_retrieval_fanout_budgets();
        if let Some(budget) = self.about_entity.derived_memory {
            upsert_fanout_budget(
                &mut budgets,
                RelationType::About,
                ObjectType::DerivedMemory,
                budget,
            );
        }
        if let Some(budget) = self.participant_entity.episode {
            upsert_fanout_budget(
                &mut budgets,
                RelationType::Involves,
                ObjectType::Episode,
                budget,
            );
        }
        if let Some(budget) = self.part_of_thread.derived_memory {
            upsert_fanout_budget(
                &mut budgets,
                RelationType::PartOfThread,
                ObjectType::DerivedMemory,
                budget,
            );
        }
        budgets
    }
}

fn reject_unsupported_fanout_target(
    name: &'static str,
    budget: Option<FanoutBudgetSettings>,
) -> Result<(), CustomError> {
    if budget.is_some() {
        return Err(ConfigValidationError {
            keys: vec![name],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "an implemented retrieval fanout target",
                actual: name.to_owned(),
            },
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
impl Settings {
    pub fn new_for_tests(
        qdrant_connection_string: SecretString,
        oxigraph_path: SecretString,
        openai_api_key: SecretString,
        embedding_model: SecretString,
    ) -> Self {
        Settings {
            qdrant_connection_string,
            oxigraph_path,
            openai_api_key,
            embedding_model,
            graph_store_mode: GraphStoreMode::InMemory,
            retrieval_stats_store_mode: RetrievalStatsStoreMode::InMemory,
            retrieval_stats_path: default_retrieval_stats_path(),
            retrieval_stats_health_fail_mode: RetrievalStatsHealthFailMode::Conservative,
            selectivity_smoothing_alpha: default_selectivity_smoothing_alpha(),
            selectivity_gamma: default_selectivity_gamma(),
            retrieval: RetrievalSettings::default(),
        }
    }
}

fn default_retrieval_stats_path() -> PathBuf {
    PathBuf::from("./data/retrieval-stats.sqlite3")
}

fn default_selectivity_smoothing_alpha() -> f64 {
    1.0
}

fn default_selectivity_gamma() -> f64 {
    1.0
}

pub(crate) fn default_retrieval_fanout_budgets(
) -> Vec<(RelationType, ObjectType, FanoutBudgetSettings)> {
    vec![
        (
            RelationType::About,
            ObjectType::DerivedMemory,
            FanoutBudgetSettings::new(0, 20),
        ),
        (
            RelationType::Involves,
            ObjectType::Episode,
            FanoutBudgetSettings::new(0, 5),
        ),
        (
            RelationType::PartOfThread,
            ObjectType::DerivedMemory,
            FanoutBudgetSettings::new(0, 15),
        ),
    ]
}

fn upsert_fanout_budget(
    budgets: &mut Vec<(RelationType, ObjectType, FanoutBudgetSettings)>,
    relation: RelationType,
    object_type: ObjectType,
    budget: FanoutBudgetSettings,
) {
    if let Some((_, _, existing)) =
        budgets
            .iter_mut()
            .find(|(item_relation, item_object_type, _)| {
                *item_relation == relation && *item_object_type == object_type
            })
    {
        *existing = budget;
    } else {
        budgets.push((relation, object_type, budget));
    }
}

fn validate_positive_f64(name: &'static str, value: f64) -> Result<(), CustomError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ConfigValidationError {
            keys: vec![name],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "a finite positive number",
                actual: value.to_string(),
            },
        }
        .into());
    }
    Ok(())
}

fn validate_fanout_budget(
    name: &'static str,
    budget: FanoutBudgetSettings,
) -> Result<(), CustomError> {
    if budget.min > budget.max {
        return Err(ConfigValidationError {
            keys: vec![name],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "min <= max",
                actual: format!("min={} max={}", budget.min, budget.max),
            },
        }
        .into());
    }
    Ok(())
}

fn fanout_budget_name(relation: RelationType, object_type: ObjectType) -> &'static str {
    match (relation, object_type) {
        (RelationType::About, ObjectType::DerivedMemory) => {
            "retrieval.fanout.about_entity.derived_memory"
        }
        (RelationType::Involves, ObjectType::Episode) => {
            "retrieval.fanout.participant_entity.episode"
        }
        (RelationType::PartOfThread, ObjectType::DerivedMemory) => {
            "retrieval.fanout.part_of_thread.derived_memory"
        }
        _ => unreachable!("unsupported configured fanout target: {relation:?} {object_type:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::CustomError;

    #[test]
    fn test_settings_new_success() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::new(external_config);
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert_eq!(settings.get_qdrant_connection(), "external_qdrant");
        assert_eq!(
            settings.get_oxigraph_path().unwrap(),
            PathBuf::from("external_oxigraph")
        );
        assert_eq!(settings.get_graph_store_mode(), GraphStoreMode::Persistent);
        assert_eq!(
            settings.get_retrieval_stats_store_mode(),
            RetrievalStatsStoreMode::Sqlite
        );
        assert_eq!(
            settings.get_retrieval_stats_path(),
            Path::new("./data/retrieval-stats.sqlite3")
        );
        assert_eq!(
            settings.get_retrieval_stats_health_fail_mode(),
            RetrievalStatsHealthFailMode::Conservative
        );
        assert_eq!(settings.get_selectivity_smoothing_alpha(), 1.0);
        assert_eq!(settings.get_selectivity_gamma(), 1.0);
        assert_eq!(
            settings.get_retrieval_fanout_budgets(),
            default_retrieval_fanout_budgets()
        );
    }

    #[test]
    fn test_settings_new_accepts_in_memory_graph_override() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("graph_store_mode", "in_memory")
            .unwrap()
            .build()
            .unwrap();

        let settings = Settings::new(external_config).unwrap();

        assert_eq!(settings.get_graph_store_mode(), GraphStoreMode::InMemory);
    }

    #[test]
    fn test_settings_new_accepts_embedded_persistent_graph_override() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "./data/oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("graph_store_mode", "persistent")
            .unwrap()
            .build()
            .unwrap();

        let settings = Settings::new(external_config).unwrap();

        assert_eq!(settings.get_graph_store_mode(), GraphStoreMode::Persistent);
        assert_eq!(
            settings.get_oxigraph_path().unwrap(),
            PathBuf::from("./data/oxigraph")
        );
    }

    #[test]
    fn test_settings_new_rejects_unknown_graph_mode() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "./data/oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("graph_store_mode", "service")
            .unwrap()
            .build()
            .unwrap();

        let error = Settings::new(external_config).unwrap_err();
        let CustomError::ConfigParseError(message) = error else {
            panic!("expected configuration parse error");
        };

        assert!(message.contains("GRAPH_STORE_MODE"));
        assert!(message.contains("persistent"));
        assert!(message.contains("in_memory"));
    }

    #[test]
    fn persistent_default_rejects_endpoint_url() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "http://localhost:7878")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .build()
            .unwrap();

        let settings = Settings::new(external_config).unwrap();
        let error = settings.get_oxigraph_path().unwrap_err();
        let CustomError::ConfigValidation(ConfigValidationError { keys, reason }) = error else {
            panic!("expected configuration validation error");
        };

        assert_eq!(settings.get_graph_store_mode(), GraphStoreMode::Persistent);
        assert_eq!(keys, vec!["OXIGRAPH_PATH"]);
        assert_eq!(
            reason,
            ConfigValidationReason::OutOfDomain {
                expected: "a local filesystem path",
                actual: "http://localhost:7878".to_owned(),
            }
        );
    }

    #[test]
    fn test_settings_new_accepts_retrieval_stats_overrides() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("retrieval_stats_store_mode", "in_memory")
            .unwrap()
            .set_override("retrieval_stats_path", "./tmp/stats.sqlite3")
            .unwrap()
            .set_override("retrieval_stats_health_fail_mode", "conservative")
            .unwrap()
            .set_override("selectivity_smoothing_alpha", 2.0)
            .unwrap()
            .set_override("selectivity_gamma", 0.5)
            .unwrap()
            .build()
            .unwrap();

        let settings = Settings::new(external_config).unwrap();

        assert_eq!(
            settings.get_retrieval_stats_store_mode(),
            RetrievalStatsStoreMode::InMemory
        );
        assert_eq!(
            settings.get_retrieval_stats_path(),
            Path::new("./tmp/stats.sqlite3")
        );
        assert_eq!(settings.get_selectivity_smoothing_alpha(), 2.0);
        assert_eq!(settings.get_selectivity_gamma(), 0.5);
    }

    #[test]
    fn test_settings_new_accepts_nested_retrieval_fanout_overrides() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("retrieval.fanout.about_entity.derived_memory.min", 2)
            .unwrap()
            .set_override("retrieval.fanout.about_entity.derived_memory.max", 8)
            .unwrap()
            .set_override("retrieval.fanout.participant_entity.episode.min", 1)
            .unwrap()
            .set_override("retrieval.fanout.participant_entity.episode.max", 3)
            .unwrap()
            .build()
            .unwrap();

        let settings = Settings::new(external_config).unwrap();

        assert_eq!(
            settings.get_retrieval_fanout_budgets(),
            vec![
                (
                    RelationType::About,
                    ObjectType::DerivedMemory,
                    FanoutBudgetSettings::new(2, 8),
                ),
                (
                    RelationType::Involves,
                    ObjectType::Episode,
                    FanoutBudgetSettings::new(1, 3),
                ),
                (
                    RelationType::PartOfThread,
                    ObjectType::DerivedMemory,
                    FanoutBudgetSettings::new(0, 15),
                ),
            ]
        );
    }

    #[test]
    fn test_settings_new_rejects_invalid_retrieval_fanout_budget() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_path", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .set_override("retrieval.fanout.about_entity.derived_memory.min", 9)
            .unwrap()
            .set_override("retrieval.fanout.about_entity.derived_memory.max", 8)
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::new(external_config);

        let Err(CustomError::ConfigValidation(error)) = result else {
            panic!("expected configuration validation error");
        };
        assert_eq!(
            error,
            ConfigValidationError {
                keys: vec!["retrieval.fanout.about_entity.derived_memory"],
                reason: ConfigValidationReason::OutOfDomain {
                    expected: "min <= max",
                    actual: "min=9 max=8".to_owned(),
                },
            }
        );
    }

    #[test]
    fn test_settings_new_rejects_unsupported_retrieval_fanout_targets() {
        for target in [
            "retrieval.fanout.about_entity.episode",
            "retrieval.fanout.participant_entity.derived_memory",
            "retrieval.fanout.part_of_thread.episode",
        ] {
            let external_config = Config::builder()
                .set_override("qdrant_connection_string", "external_qdrant")
                .unwrap()
                .set_override("oxigraph_path", "external_oxigraph")
                .unwrap()
                .set_override("openai_api_key", "external_openai")
                .unwrap()
                .set_override("embedding_model", "TextEmbedding3Small")
                .unwrap()
                .set_override(format!("{target}.min"), 0)
                .unwrap()
                .set_override(format!("{target}.max"), 1)
                .unwrap()
                .build()
                .unwrap();

            let result = Settings::new(external_config);

            let Err(CustomError::ConfigValidation(error)) = result else {
                panic!("expected configuration validation error for {target}");
            };
            assert_eq!(
                error,
                ConfigValidationError {
                    keys: vec![target],
                    reason: ConfigValidationReason::OutOfDomain {
                        expected: "an implemented retrieval fanout target",
                        actual: target.to_owned(),
                    },
                },
                "{target} should be rejected",
            );
        }
    }

    #[test]
    fn test_settings_new_rejects_invalid_selectivity_numbers() {
        for (key, value) in [
            ("selectivity_smoothing_alpha", 0.0),
            ("selectivity_smoothing_alpha", -1.0),
            ("selectivity_smoothing_alpha", f64::INFINITY),
            ("selectivity_smoothing_alpha", f64::NAN),
            ("selectivity_gamma", 0.0),
            ("selectivity_gamma", -1.0),
            ("selectivity_gamma", f64::INFINITY),
            ("selectivity_gamma", f64::NAN),
        ] {
            let external_config = Config::builder()
                .set_override("qdrant_connection_string", "external_qdrant")
                .unwrap()
                .set_override("oxigraph_path", "external_oxigraph")
                .unwrap()
                .set_override("openai_api_key", "external_openai")
                .unwrap()
                .set_override("embedding_model", "TextEmbedding3Small")
                .unwrap()
                .set_override(key, value)
                .unwrap()
                .build()
                .unwrap();

            let result = Settings::new(external_config);

            assert!(
                matches!(
                    result,
                    Err(CustomError::ConfigValidation(ConfigValidationError {
                        keys,
                        reason: ConfigValidationReason::OutOfDomain {
                            expected: "a finite positive number",
                            ..
                        },
                    })) if keys == vec![key]
                ),
                "{key}={value:?} should be rejected",
            );
        }
    }

    #[test]
    fn test_settings_new_error() {
        let incomplete_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::new(incomplete_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }
}
