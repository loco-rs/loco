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
use tracing::info;

mod auth;
mod cache;
mod database;
mod logger;
mod mailer;
mod queue;
mod server;
mod template;

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
    /// ```rust,ignore
    /// use loco_rs::{config::Config, environment::Environment};
    ///
    /// let config = Config::new(&Environment::Development)?;
    /// ```
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
    /// ```rust,ignore
    /// use loco_rs::{config::Config, environment::Environment};
    /// use std::path::Path;
    ///
    /// let config = Config::from_folder(&Environment::Development, Path::new("config"))?;
    /// ```
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

    /// Reads a single YAML config file, renders its template tags, and parses
    /// it into a [`serde_yaml::Value`].
    ///
    /// Templating uses the YAML-safe `<%= ... %>` delimiters (see
    /// [`template`]); Tera's native `{{ ... }}` still works but is deprecated.
    ///
    /// # Errors
    /// Returns an error naming `path` when the file cannot be read, rendered,
    /// or parsed as YAML.
    fn load_yaml_value(path: &Path) -> Result<serde_yaml::Value> {
        let content = fs::read_to_string(path)?;
        let rendered = template::render(&content)?;

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

    /// The unit tests beside `JWTLocationConfig` parse the enum on its own.
    /// This drives the whole loader — file read, `<%= %>` template pass, merge,
    /// typed deserialization — because the hand-written `Deserialize` uses
    /// `deserialize_any`, and a self-describing format is a precondition for
    /// that. If the loader ever routes config through a non-self-describing
    /// path, this is what notices.
    #[test]
    fn a_jwt_cookie_location_survives_the_whole_config_loader() {
        let yaml = format!(
            "{BASE_YAML}auth:\n  jwt:\n    secret: shh\n    expiration: 604800\n    \
             location:\n      from: Cookie\n      name: auth_token\n"
        );

        let tree = TreeBuilder::default()
            .drop(true)
            .add_file("test.yaml", &yaml)
            .create()
            .expect("create temp config folder");

        let config =
            Config::from_folder(&Environment::Test, &tree.root).expect("load config with auth");

        let jwt = config.auth.expect("auth present").jwt.expect("jwt present");

        assert!(matches!(
            jwt.location.expect("location present"),
            JWTLocationConfig::Single(JWTLocation::Cookie { name }) if name == "auth_token"
        ));
    }

    /// And the error a real config produces names the field at fault.
    #[test]
    fn a_bad_jwt_location_in_a_real_config_names_the_problem() {
        let yaml = format!(
            "{BASE_YAML}auth:\n  jwt:\n    secret: shh\n    expiration: 604800\n    \
             location:\n      from: cookie\n      name: auth_token\n"
        );

        let tree = TreeBuilder::default()
            .drop(true)
            .add_file("test.yaml", &yaml)
            .create()
            .expect("create temp config folder");

        let err = Config::from_folder(&Environment::Test, &tree.root)
            .expect_err("`cookie` is not a variant");
        let message = err.to_string();

        assert!(
            message.contains("unknown variant") && message.contains("Cookie"),
            "the loader should surface the inner error, got: {message}"
        );
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

    /// End-to-end guard for <https://github.com/loco-rs/loco/issues/1727>: a
    /// config written with the YAML-safe `<% %>` delimiters must (a) be valid
    /// YAML before rendering, so formatters cannot corrupt it, and (b) still
    /// resolve environment variables into correctly *typed* fields.
    #[test]
    fn yaml_safe_templates_are_valid_yaml_and_resolve_typed_env_values() {
        // Deliberately unique names so parallel tests cannot collide.
        const PORT_VAR: &str = "LOCO_CFG_TEST_PORT_1727";
        const LOGGING_VAR: &str = "LOCO_CFG_TEST_DB_LOGGING_1727";

        let yaml = format!(
            r#"
logger:
  enable: false
  level: <%= get_env(name="LOCO_CFG_TEST_LEVEL_1727", default="info") %>
  format: compact
server:
  port: <%= get_env(name="{PORT_VAR}", default="5150") %>
  host: localhost
mailer:
  stub: true
database:
  uri: "sqlite::memory:"
  enable_logging: <%= get_env(name="{LOGGING_VAR}", default="false") %>
  connect_timeout: <%= get_env(name="LOCO_CFG_TEST_CT_1727", default="500") %>
  idle_timeout: 500
  min_connections: 1
  max_connections: 1
  auto_migrate: false
  dangerously_truncate: false
  dangerously_recreate: false
"#
        );

        // (a) The file parses as ordinary YAML *before* any rendering, with the
        // templates sitting in plain string scalars.
        let raw: serde_yaml::Value =
            serde_yaml::from_str(&yaml).expect("templated config must be valid YAML at rest");
        assert!(raw["server"]["port"].is_string());

        // (b) Rendering resolves env vars and YAML re-types the bare results.
        // SAFETY: these variable names are unique to this test, so no other
        // thread reads or writes them concurrently.
        unsafe {
            std::env::set_var(PORT_VAR, "8080");
            std::env::set_var(LOGGING_VAR, "true");
        }

        let tree = TreeBuilder::default()
            .drop(true)
            .add("test.yaml", &yaml)
            .create()
            .unwrap();
        let config = Config::from_folder(&Environment::Test, &tree.root).unwrap();

        // SAFETY: see above — names are unique to this test.
        unsafe {
            std::env::remove_var(PORT_VAR);
            std::env::remove_var(LOGGING_VAR);
        }

        assert_eq!(
            config.server.port, 8080,
            "env var must override the default"
        );
        assert!(
            config.database.enable_logging,
            "a bool field must resolve from the environment as a real bool"
        );
        // Untouched vars fall back to their typed defaults.
        assert_eq!(config.database.connect_timeout, 500);
    }
}
