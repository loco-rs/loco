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

pub const DEFAULT_ENVIRONMENT: &str = "development";
pub const LOCO_ENV: &str = "LOCO_ENV";
pub const RAILS_ENV: &str = "RAILS_ENV";
pub const NODE_ENV: &str = "NODE_ENV";

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
    /// Returns error if an error occurs during loading
    /// configuration file an parse into [`Config`] struct.
    pub fn load(&self) -> Result<Config> {
        Config::new(self)
    }

    /// Load environment variables from the given config path
    ///
    /// # Errors
    ///
    /// Returns error if an error occurs during loading
    /// configuration file an parse into [`Config`] struct.
    pub fn load_from_folder<P: AsRef<Path>>(&self, path: P) -> Result<Config> {
        Config::from_folder(self, path)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any(s) => s.fmt(f),
            _ => to_variant_name(self).expect("only enum supported").fmt(f),
        }
    }
}

impl FromStr for Environment {
    type Err = &'static str;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "production" => Ok(Self::Production),
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            s => Ok(Self::Any(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use rstest::rstest;

    use super::*;
    #[test]
    fn test_resolve_env() {
        let original = env::var("LOCO_ENV");

        env::remove_var(LOCO_ENV);
        env::remove_var(RAILS_ENV);
        env::remove_var(NODE_ENV);
        assert_eq!(resolve_from_env(), "development");
        env::set_var("LOCO_ENV", "custom");
        assert_eq!(resolve_from_env(), "custom");

        if let Ok(v) = original {
            env::set_var(LOCO_ENV, v);
        }
    }

    #[test]
    fn test_display() {
        assert_eq!("production", Environment::Production.to_string());
        assert_eq!("custom", Environment::Any("custom".to_string()).to_string());
    }

    #[test]
    fn test_into() {
        let e: Environment = "production".to_string().into();
        assert_eq!(e, Environment::Production);
        let e: Environment = "custom".to_string().into();
        assert_eq!(e, Environment::Any("custom".to_string()));
    }

    #[test]
    fn test_from_folder() {
        let config = Environment::Development.load();
        assert!(config.is_ok());
    }

    #[test]
    fn test_config_local_override() {
        let config = Environment::Development.load().unwrap();
        let binding = config.settings.unwrap();
        assert_eq!(
            binding.get("from").unwrap().as_str(),
            Some("development.local.yaml")
        );
    }

    #[rstest]
    #[case(Environment::Development)]
    #[case(Environment::Production)]
    #[case(Environment::Test)]
    #[case(Environment::Any("custom".to_string()))]
    #[test]
    fn can_load_config(#[case] env: Environment) {
        assert!(env.load().is_ok());
    }
}
