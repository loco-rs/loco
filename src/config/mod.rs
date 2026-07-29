//! # Configuration Management
//!
//! This module defines the configuration structures and functions to manage and
//! load configuration settings for the application.

/***
=============
CONTRIBUTORS:
=============

Here's a check list when adding configuration values:

* Add the new configuration piece
* Document each field with the appropriate rustdoc comment
* Go to `starters/`, evaluate which starter needs a configuration update, and update as needed.
  apply a YAML comment above the new field or section with explanation and possible values.

Notes:
* Configuration is feature-dependent: with and without database
* Configuration is "stage" dependent: development, test, production
* We typically provide best practice values for development and test, but by-design we do not provide default values for production

***/
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

mod auth;
mod cache;
mod database;
mod logger;
mod mailer;
mod queue;
mod server;

pub use auth::*;
pub use cache::*;
pub use database::*;
pub use logger::*;
pub use mailer::*;
pub use queue::*;
pub use server::*;

use crate::{environment::Environment, scheduler, Error, Result};

static DEFAULT_FOLDER: OnceLock<PathBuf> = OnceLock::new();

fn get_default_folder() -> &'static PathBuf {
    DEFAULT_FOLDER.get_or_init(|| PathBuf::from("config"))
}
/// Main application configuration structure.
///
/// This struct encapsulates various configuration settings. The configuration
/// can be customized through YAML files for different environments.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
    #[cfg(feature = "with-db")]
    pub database: Database,
    #[serde(default)]
    pub cache: CacheConfig,
    pub queue: Option<QueueConfig>,
    pub auth: Option<Auth>,
    #[serde(default)]
    pub workers: Workers,
    pub mailer: Option<Mailer>,
    pub initializers: Option<Initializers>,

    /// Custom app settings
    ///
    /// Example:
    /// ```yaml
    /// settings:
    ///   allow_list:
    ///     - google.com
    ///     - apple.com
    /// ```
    /// And then optionally deserialize it to your own `Settings` type by
    /// accessing `ctx.config.settings`.
    #[serde(default)]
    pub settings: Option<serde_json::Value>,

    pub scheduler: Option<scheduler::Config>,
}

/// Initializers configuration
///
/// Example (development): To configure settings for oauth2 or custom view
/// engine
/// ```yaml
/// # config/development.yaml
/// initializers:
///  oauth2:
///   authorization_code: # Authorization code grant type
///     - client_identifier: google # Identifier for the `OAuth2` provider.
///       Replace 'google' with your provider's name if different, must be
///       unique within the oauth2 config. ... # other fields
pub type Initializers = BTreeMap<String, serde_json::Value>;

impl Config {
    /// Creates a new configuration instance based on the specified environment.
    ///
    /// # Errors
    ///
    /// Returns error when could not convert the give path to
    /// [`Config`] struct.
    ///
    /// # Example
    ///
    /// ```rust
    /// use loco_rs::{
    ///     config::Config,
    ///     environment::Environment,
    /// };
    ///
    /// #[tokio::main]
    /// async fn load(environment: &Environment) -> Config {
    ///     Config::new(environment).expect("configuration loading")
    /// }
    pub fn new(env: &Environment) -> Result<Self> {
        let config = Self::from_folder(env, get_default_folder().as_path())?;
        Ok(config)
    }

    /// Loads configuration settings from a folder for the specified
    /// environment.
    ///
    /// # Errors
    /// Returns error when could not convert the give path to
    /// [`Config`] struct.
    ///
    /// # Example
    ///
    /// ```rust
    /// use loco_rs::{
    ///     config::Config,
    ///     environment::Environment,
    /// };
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn load(environment: &Environment) -> Config{
    ///     Config::from_folder(environment, &PathBuf::from("config")).expect("configuration loading")
    /// }
    pub fn from_folder(env: &Environment, path: &Path) -> Result<Self> {
        // `{env}.yaml` is the base configuration, `{env}.local.yaml` is an
        // optional, git-ignorable override layered on top of it. When both
        // exist they are deep-merged (local wins), rather than one file
        // replacing the other entirely.
        let base_path = path.join(format!("{env}.yaml"));
        let local_path = path.join(format!("{env}.local.yaml"));

        let base_exists = base_path.exists();
        let local_exists = local_path.exists();

        let (merged, selected_path_display) = match (base_exists, local_exists) {
            (false, false) => {
                return Err(Error::Message(format!(
                    "no configuration file found in folder: {}",
                    path.display()
                )));
            }
            (true, false) => (
                Self::load_yaml_value(&base_path)?,
                base_path.display().to_string(),
            ),
            (false, true) => (
                Self::load_yaml_value(&local_path)?,
                local_path.display().to_string(),
            ),
            (true, true) => {
                let base = Self::load_yaml_value(&base_path)?;
                let local = Self::load_yaml_value(&local_path)?;
                (
                    merge_yaml(base, local),
                    format!(
                        "{} (merged with {})",
                        base_path.display(),
                        local_path.display()
                    ),
                )
            }
        };

        info!(
            selected_path = selected_path_display,
            "loading environment from"
        );

        serde_yaml::from_value(merged).map_err(|err| Error::YAMLFile(err, selected_path_display))
    }

    /// Reads a single YAML config file, renders it through Tera, and parses
    /// it into a [`serde_yaml::Value`].
    ///
    /// # Errors
    /// Returns an error naming `path` when the file cannot be read, rendered,
    /// or parsed as YAML.
    fn load_yaml_value(path: &Path) -> Result<serde_yaml::Value> {
        let content = fs::read_to_string(path)?;
        let rendered = crate::tera::render_string(&content, &json!({}))?;

        serde_yaml::from_str(&rendered)
            .map_err(|err| Error::YAMLFile(err, path.to_string_lossy().to_string()))
    }

    /// Get a reference to the JWT configuration.
    ///
    /// # Errors
    /// return an error when jwt token not configured
    pub fn get_jwt_config(&self) -> Result<&JWT> {
        self.auth
            .as_ref()
            .and_then(|auth| auth.jwt.as_ref())
            .map_or_else(
                || Err(Error::Any("no JWT config found".to_string().into())),
                Ok,
            )
    }
}

/// Deep-merges two YAML values, with `override_` taking precedence over
/// `base`.
///
/// * When both sides are mappings, the merge recurses key-by-key: keys
///   present only in `base` are kept, keys present only in `override_` are
///   added, and keys present in both are merged recursively.
/// * For any other node type (scalars, sequences, or a mismatch between a
///   mapping and a non-mapping), `override_` fully replaces `base` — in
///   particular, sequences are replaced wholesale, never concatenated.
fn merge_yaml(base: serde_yaml::Value, override_: serde_yaml::Value) -> serde_yaml::Value {
    match (base, override_) {
        (serde_yaml::Value::Mapping(mut base_map), serde_yaml::Value::Mapping(override_map)) => {
            for (key, override_value) in override_map {
                let merged_value = match base_map.remove(&key) {
                    Some(base_value) => merge_yaml(base_value, override_value),
                    None => override_value,
                };
                base_map.insert(key, merged_value);
            }
            serde_yaml::Value::Mapping(base_map)
        }
        (_, override_) => override_,
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = serde_yaml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}

#[cfg(test)]
mod tests {
    use tree_fs::TreeBuilder;

    use super::*;
    use crate::environment::Environment;

    /// A minimal, valid `test.yaml`/`test.local.yaml` body. `settings` is
    /// used as a free-form nested map to exercise the merge behavior beyond
    /// what the typed `Config` fields allow.
    const BASE_YAML: &str = r#"
logger:
  enable: false
  level: info
  format: compact
server:
  port: 5150
  host: localhost
mailer:
  stub: true
database:
  uri: "sqlite::memory:"
  enable_logging: false
  connect_timeout: 500
  idle_timeout: 500
  min_connections: 1
  max_connections: 1
  auto_migrate: false
  dangerously_truncate: false
  dangerously_recreate: false
settings:
  base_only: base
  shared_map:
    a: base_a
    b: base_b
  seq:
    - 1
    - 2
    - 3
"#;

    const LOCAL_YAML: &str = r"
server:
  port: 6000
settings:
  local_only: local
  shared_map:
    b: local_b
    c: local_c
  seq:
    - 9
";

    #[test]
    fn base_and_local_are_deep_merged_with_local_precedence() {
        let tree = TreeBuilder::default()
            .drop(true)
            .add_file("test.yaml", BASE_YAML)
            .add_file("test.local.yaml", LOCAL_YAML)
            .create()
            .expect("create temp config folder");

        let config =
            Config::from_folder(&Environment::Test, &tree.root).expect("merge base and local");

        // local scalar overrides base scalar
        assert_eq!(config.server.port, 6000);
        // a nested key present only in base (server.host) survives because
        // local only overrides `port`
        assert_eq!(config.server.host, "localhost");

        let settings = config.settings.expect("settings present");
        // key only in base survives
        assert_eq!(settings["base_only"], "base");
        // key only in local is added
        assert_eq!(settings["local_only"], "local");
        // nested map: base-only key survives, shared key is overridden,
        // local-only key is added
        assert_eq!(settings["shared_map"]["a"], "base_a");
        assert_eq!(settings["shared_map"]["b"], "local_b");
        assert_eq!(settings["shared_map"]["c"], "local_c");
        // sequences are replaced wholesale, never concatenated
        assert_eq!(settings["seq"], serde_json::json!([9]));
    }

    #[test]
    fn only_base_present_uses_base() {
        let tree = TreeBuilder::default()
            .drop(true)
            .add_file("test.yaml", BASE_YAML)
            .create()
            .expect("create temp config folder");

        let config =
            Config::from_folder(&Environment::Test, &tree.root).expect("load base-only config");

        assert_eq!(config.server.port, 5150);
        assert_eq!(
            config.settings.expect("settings present")["base_only"],
            "base"
        );
    }

    #[test]
    fn only_local_present_uses_local() {
        let tree = TreeBuilder::default()
            .drop(true)
            .add_file("test.local.yaml", BASE_YAML)
            .create()
            .expect("create temp config folder");

        let config =
            Config::from_folder(&Environment::Test, &tree.root).expect("load local-only config");

        assert_eq!(config.server.port, 5150);
        assert_eq!(
            config.settings.expect("settings present")["base_only"],
            "base"
        );
    }

    #[test]
    fn neither_file_present_returns_error() {
        let tree = TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create empty temp config folder");

        let err = Config::from_folder(&Environment::Test, &tree.root)
            .expect_err("should error when no config file exists");

        assert!(matches!(err, Error::Message(_)));
    }

    #[test]
    fn merge_yaml_replaces_sequences_instead_of_concatenating() {
        let base: serde_yaml::Value = serde_yaml::from_str("seq: [1, 2, 3]").unwrap();
        let over: serde_yaml::Value = serde_yaml::from_str("seq: [9]").unwrap();

        let merged = merge_yaml(base, over);

        assert_eq!(
            merged,
            serde_yaml::from_str::<serde_yaml::Value>("seq: [9]").unwrap()
        );
    }

    #[test]
    fn merge_yaml_recurses_into_nested_maps() {
        let base: serde_yaml::Value = serde_yaml::from_str(
            r"
a: base_a
nested:
  x: base_x
  y: base_y
",
        )
        .unwrap();
        let over: serde_yaml::Value = serde_yaml::from_str(
            r"
nested:
  y: local_y
  z: local_z
",
        )
        .unwrap();

        let merged = merge_yaml(base, over);

        let expected: serde_yaml::Value = serde_yaml::from_str(
            r"
a: base_a
nested:
  x: base_x
  y: local_y
  z: local_z
",
        )
        .unwrap();

        assert_eq!(merged, expected);
    }
}
