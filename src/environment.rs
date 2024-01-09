//! Defines the application environment.
//! By given the environment you can also load the application configuration
//!
//! # Example:
//!
//! ```rust
//! use std::str::FromStr;
//! use loco_rs::environment::Environment;
//!
//! pub fn load(environment: &str) {
//!  let environment = Environment::from_str(environment).unwrap_or(Environment::Any(environment.to_string()));
//!  let config = environment.load().expect("failed to load environment");
//! }
//! ```
use std::{path::Path, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;

use super::config::Config;
use crate::Result;

const DEFAULT_ENVIRONMENT: &str = "development";

impl From<String> for Environment {
    fn from(env: String) -> Self {
        Self::from_str(&env).unwrap_or(Self::Any(env))
    }
}
#[must_use]
pub fn resolve_from_env() -> String {
    std::env::var("LOCO_ENV")
        .or_else(|_| std::env::var("RAILS_ENV"))
        .or_else(|_| std::env::var("NODE_ENV"))
        .unwrap_or_else(|_| DEFAULT_ENVIRONMENT.to_string())
}

/// Application environment
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Environment {
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "test")]
    Test,
    Any(String),
}

impl Environment {
    /// Load environment variables from local configuration
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] if an error occurs during loading
    /// configuration file an parse into [`Config`] struct.
    pub fn load(&self) -> Result<Config> {
        Config::new(self)
    }

    /// Load environment variables from the given config path
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] if an error occurs during loading
    /// configuration file an parse into [`Config`] struct.
    pub fn load_from_folder(&self, path: &Path) -> Result<Config> {
        Config::from_folder(self, path)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}

impl FromStr for Environment {
    type Err = &'static str;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "production" => Ok(Self::Production),
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            _ => Err(
                " error parsing environment: expected one of  \"production\", \"development\", \
                 \"test\" or any environment that has config file",
            ),
        }
    }
}
