use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};

use crate::api::types::{ObjectType, RelationType};
use crate::errors::CustomError;
use crate::internal::models::vector::EmbeddingModel;

#[derive(Debug, Deserialize)]
pub struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_connection_string: SecretString,
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

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphStoreMode {
    #[default]
    Service,
    Persistent,
    InMemory,
}

impl GraphStoreMode {
    fn parse(value: &str) -> Result<Self, CustomError> {
        match value {
            "service" => Ok(Self::Service),
            "persistent" => Ok(Self::Persistent),
            "in_memory" => Ok(Self::InMemory),
            other => Err(CustomError::ConfigParseError(format!(
                "GRAPH_STORE_MODE must be service, persistent, or in_memory, got {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStatsStoreMode {
    #[default]
    Sqlite,
    InMemory,
}

impl RetrievalStatsStoreMode {
    fn parse(value: &str) -> Result<Self, CustomError> {
        match value {
            "sqlite" => Ok(Self::Sqlite),
            "in_memory" => Ok(Self::InMemory),
            other => Err(CustomError::ConfigParseError(format!(
                "RETRIEVAL_STATS_STORE_MODE must be sqlite or in_memory, got {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStatsHealthFailMode {
    #[default]
    Conservative,
}

impl RetrievalStatsHealthFailMode {
    fn parse(value: &str) -> Result<Self, CustomError> {
        match value {
            "conservative" => Ok(Self::Conservative),
            other => Err(CustomError::ConfigParseError(format!(
                "RETRIEVAL_STATS_HEALTH_FAIL_MODE must be conservative, got {other}"
            ))),
        }
    }
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
    ///     - `oxigraph_connection_string`: Connection string for Oxigraph database
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

    /// Loads settings from environment variables and configuration files using default loaders.
    ///
    /// # Description
    ///
    /// This function provides a convenient way to load settings using the default environment and configuration loaders.
    /// If a `.env` file exists in the project root it will be loaded automatically.
    /// When the file is absent the function relies solely on the current environment variables.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used anywhere else in the codebase.
    /// For production code, use the `Settings::new()` constructor with an explicitly configured `Config` instance.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment and config files
    /// - `Err`: A `CustomError` if:
    ///     - Environment variables are missing or invalid
    ///     - There are errors parsing the configuration
    ///
    pub(crate) fn load() -> Result<Self, CustomError> {
        dotenvy::dotenv().ok();

        let qdrant_connection_string = env::var("QDRANT_CONNECTION_STRING")
            .map_err(|e| CustomError::ConfigParseError(format!("QDRANT_CONNECTION_STRING: {e}")))?;
        let oxigraph_connection_string = env::var("OXIGRAPH_CONNECTION_STRING").map_err(|e| {
            CustomError::ConfigParseError(format!("OXIGRAPH_CONNECTION_STRING: {e}"))
        })?;
        let openai_api_key = env::var("OPENAI_API_KEY")
            .map_err(|e| CustomError::ConfigParseError(format!("OPENAI_API_KEY: {e}")))?;
        let embedding_model = env::var("EMBEDDING_MODEL")
            .map_err(|e| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {e}")))?;
        let graph_store_mode = env::var("GRAPH_STORE_MODE")
            .map(|value| GraphStoreMode::parse(&value))
            .unwrap_or(Ok(GraphStoreMode::Service))?;
        let retrieval_stats_store_mode = env::var("RETRIEVAL_STATS_STORE_MODE")
            .map(|value| RetrievalStatsStoreMode::parse(&value))
            .unwrap_or(Ok(RetrievalStatsStoreMode::Sqlite))?;
        let retrieval_stats_path = env::var("RETRIEVAL_STATS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_retrieval_stats_path());
        let retrieval_stats_health_fail_mode = env::var("RETRIEVAL_STATS_HEALTH_FAIL_MODE")
            .map(|value| RetrievalStatsHealthFailMode::parse(&value))
            .unwrap_or(Ok(RetrievalStatsHealthFailMode::Conservative))?;
        let selectivity_smoothing_alpha = env::var("SELECTIVITY_SMOOTHING_ALPHA")
            .map(|value| parse_positive_f64("SELECTIVITY_SMOOTHING_ALPHA", &value))
            .unwrap_or(Ok(default_selectivity_smoothing_alpha()))?;
        let selectivity_gamma = env::var("SELECTIVITY_GAMMA")
            .map(|value| parse_positive_f64("SELECTIVITY_GAMMA", &value))
            .unwrap_or(Ok(default_selectivity_gamma()))?;
        let retrieval = RetrievalSettings {
            fanout: load_env_fanout_settings()?,
        };

        let settings = Self {
            qdrant_connection_string: SecretString::new(qdrant_connection_string.into()),
            oxigraph_connection_string: SecretString::new(oxigraph_connection_string.into()),
            openai_api_key: SecretString::new(openai_api_key.into()),
            embedding_model: SecretString::new(embedding_model.into()),
            graph_store_mode,
            retrieval_stats_store_mode,
            retrieval_stats_path,
            retrieval_stats_health_fail_mode,
            selectivity_smoothing_alpha,
            selectivity_gamma,
            retrieval,
        };
        settings.validate_selectivity_settings()?;
        settings.validate_fanout_settings()?;
        Ok(settings)
    }

    pub fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    pub fn get_oxigraph_connection(&self) -> &str {
        self.oxigraph_connection_string.expose_secret()
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
        let configured_path = self.get_oxigraph_connection();
        if configured_path.contains("://") {
            return Err(CustomError::ConfigParseError(
                "OXIGRAPH_CONNECTION_STRING must be a filesystem path for embedded persistent graph mode"
                    .to_owned(),
            ));
        }

        let path = Path::new(configured_path);
        if path.as_os_str().is_empty() {
            return Err(CustomError::ConfigParseError(
                "OXIGRAPH_CONNECTION_STRING must be a filesystem path for persistent graph mode"
                    .to_owned(),
            ));
        }
        Ok(path.to_path_buf())
    }

    pub fn get_oxigraph_endpoint(&self) -> Result<String, CustomError> {
        let endpoint = self.get_oxigraph_connection().trim().trim_end_matches('/');
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return Err(CustomError::ConfigParseError(
                "OXIGRAPH_CONNECTION_STRING must be an HTTP(S) endpoint for service graph mode"
                    .to_owned(),
            ));
        }
        Ok(endpoint.to_owned())
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
        for (relation, object_type, budget) in self.get_retrieval_fanout_budgets() {
            validate_fanout_budget(
                &format!(
                    "retrieval.fanout.{}.{}",
                    fanout_relation_name(relation),
                    fanout_object_type_name(object_type)
                ),
                budget,
            )?;
        }
        Ok(())
    }
}

impl RetrievalFanoutSettings {
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

#[cfg(test)]
impl Settings {
    pub fn new_for_tests(
        qdrant_connection_string: SecretString,
        oxigraph_connection_string: SecretString,
        openai_api_key: SecretString,
        embedding_model: SecretString,
    ) -> Self {
        Settings {
            qdrant_connection_string,
            oxigraph_connection_string,
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

fn load_env_fanout_settings() -> Result<RetrievalFanoutSettings, CustomError> {
    Ok(RetrievalFanoutSettings {
        about_entity: FanoutObjectSettings {
            derived_memory: parse_env_fanout_budget(
                "RETRIEVAL_FANOUT_ABOUT_ENTITY_DERIVED_MEMORY_MIN",
                "RETRIEVAL_FANOUT_ABOUT_ENTITY_DERIVED_MEMORY_MAX",
            )?,
            episode: None,
        },
        participant_entity: FanoutObjectSettings {
            derived_memory: None,
            episode: parse_env_fanout_budget(
                "RETRIEVAL_FANOUT_PARTICIPANT_ENTITY_EPISODE_MIN",
                "RETRIEVAL_FANOUT_PARTICIPANT_ENTITY_EPISODE_MAX",
            )?,
        },
        part_of_thread: FanoutObjectSettings {
            derived_memory: parse_env_fanout_budget(
                "RETRIEVAL_FANOUT_PART_OF_THREAD_DERIVED_MEMORY_MIN",
                "RETRIEVAL_FANOUT_PART_OF_THREAD_DERIVED_MEMORY_MAX",
            )?,
            episode: None,
        },
    })
}

fn parse_env_fanout_budget(
    min_name: &str,
    max_name: &str,
) -> Result<Option<FanoutBudgetSettings>, CustomError> {
    let min_value = env::var(min_name).ok();
    let max_value = env::var(max_name).ok();
    match (min_value, max_value) {
        (None, None) => Ok(None),
        (Some(min_value), Some(max_value)) => {
            let budget = FanoutBudgetSettings::new(
                parse_usize(min_name, &min_value)?,
                parse_usize(max_name, &max_value)?,
            );
            validate_fanout_budget(&format!("{min_name}/{max_name}"), budget)?;
            Ok(Some(budget))
        }
        _ => Err(CustomError::ConfigParseError(format!(
            "{min_name} and {max_name} must be provided together"
        ))),
    }
}

fn parse_usize(name: &str, value: &str) -> Result<usize, CustomError> {
    value.parse::<usize>().map_err(|error| {
        CustomError::ConfigParseError(format!("{name} must be a non-negative integer: {error}"))
    })
}

fn parse_positive_f64(name: &str, value: &str) -> Result<f64, CustomError> {
    let parsed = value.parse::<f64>().map_err(|error| {
        CustomError::ConfigParseError(format!("{name} must be a finite positive number: {error}"))
    })?;
    validate_positive_f64(name, parsed).map_err(|_| {
        CustomError::ConfigParseError(format!(
            "{name} must be a finite positive number, got {value}"
        ))
    })?;
    Ok(parsed)
}

fn validate_positive_f64(name: &str, value: f64) -> Result<(), CustomError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(CustomError::ConfigParseError(format!(
            "{name} must be a finite positive number, got {value}"
        )));
    }
    Ok(())
}

fn validate_fanout_budget(name: &str, budget: FanoutBudgetSettings) -> Result<(), CustomError> {
    if budget.min > budget.max {
        return Err(CustomError::ConfigParseError(format!(
            "{name}.min must be less than or equal to {name}.max, got min={} max={}",
            budget.min, budget.max
        )));
    }
    Ok(())
}

fn fanout_relation_name(relation: RelationType) -> &'static str {
    match relation {
        RelationType::About => "about_entity",
        RelationType::Involves => "participant_entity",
        RelationType::PartOfThread => "part_of_thread",
        _ => "unsupported_relation",
    }
}

fn fanout_object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::Episode => "episode",
        _ => "unsupported_object_type",
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
            .set_override("oxigraph_connection_string", "external_oxigraph")
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
        assert_eq!(settings.get_oxigraph_connection(), "external_oxigraph");
        assert_eq!(settings.get_graph_store_mode(), GraphStoreMode::Service);
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
            .set_override("oxigraph_connection_string", "external_oxigraph")
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
            .set_override("oxigraph_connection_string", "./data/oxigraph")
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
    fn test_settings_new_accepts_retrieval_stats_overrides() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_connection_string", "external_oxigraph")
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
            .set_override("oxigraph_connection_string", "external_oxigraph")
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
            .set_override("oxigraph_connection_string", "external_oxigraph")
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

        assert!(matches!(
            result,
            Err(CustomError::ConfigParseError(message))
                if message.contains("retrieval.fanout.about_entity.derived_memory")
        ));
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
                .set_override("oxigraph_connection_string", "external_oxigraph")
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
                matches!(result, Err(CustomError::ConfigParseError(_))),
                "{key}={value:?} should be rejected"
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
