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
        // by order of precedence
        let files = [
            path.join(format!("{env}.local.yaml")),
            path.join(format!("{env}.yaml")),
        ];

        let selected_path = files.iter().find(|p| p.exists()).ok_or_else(|| {
            Error::Message(format!(
                "no configuration file found in folder: {}",
                path.display()
            ))
        })?;

        info!(selected_path =? selected_path, "loading environment from");

        let content = fs::read_to_string(selected_path)?;
        let rendered = crate::tera::render_string(&content, &json!({}))?;

        serde_yaml::from_str(&rendered)
            .map_err(|err| Error::YAMLFile(err, selected_path.to_string_lossy().to_string()))
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

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = serde_yaml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}
