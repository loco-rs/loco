//! Cargo.toml configuration reader module
//!
//! This module provides functionality to read and parse Cargo.toml files,
//! with specific support for accessing database entity configuration
//! under the `[package.metadata.db.entity]` section.

use crate::errors::Error;
use crate::Result as AppResult;
use std::path::Path;
use toml::Table;

/// Represents a parsed Cargo.toml configuration
///
/// This struct holds the parsed TOML data from a Cargo.toml file
/// and provides methods to access specific sections of the configuration.
pub struct CargoConfig {
    toml: Table,
}

impl CargoConfig {
    /// Creates a new [`CargoConfig`] by reading the Cargo.toml file from the current directory
    ///
    /// # Errors
    /// * If the Cargo.toml file cannot be read
    /// * If the file contains invalid TOML
    pub fn from_current_dir() -> AppResult<Self> {
        Self::from_path("Cargo.toml")
    }

    /// Creates a new [`CargoConfig`] by reading the Cargo.lock file from the current directory
    ///
    /// # Errors
    /// * If the Cargo.lock file cannot be read
    /// * If the file contains invalid TOML
    pub fn lock_from_current_dir() -> AppResult<Self> {
        Self::from_path("Cargo.lock")
    }

    /// Creates a new [`CargoConfig`] by reading and parsing a TOML file from the specified path
    ///
    /// # Errors
    /// * If the file cannot be read
    /// * If the file contains invalid TOML
    pub fn from_path(path: impl AsRef<Path>) -> AppResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Message(format!("Failed to read Cargo.toml: {e}")))?;

        let toml = content
            .parse::<Table>()
            .map_err(|e| Error::Message(format!("Failed to parse Cargo.toml: {e}")))?;

        Ok(Self { toml })
    }

    /// Retrieves the database entity configuration from the Cargo.toml
    ///
    /// Looks for configuration under the `[package.metadata.db.entity]` section.
    #[must_use]
    pub fn get_db_entities(&self) -> Option<&Table> {
        self.toml
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|m| m.get("db"))
            .and_then(|d| d.as_table())
            .and_then(|d| d.get("entity"))
            .and_then(|e| e.as_table())
    }

    /// Gets the package array from Cargo.lock
    ///
    /// # Errors
    /// Returns an error if the package array is missing or invalid
    pub fn get_package_array(&self) -> AppResult<&[toml::Value]> {
        self.toml
            .get("package")
            .and_then(|v| v.as_array())
            .map(std::vec::Vec::as_slice)
            .ok_or_else(|| Error::Message("Missing package array in Cargo.lock".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CARGO_TOML: &str = r#"
[package]
name = "test-app"
version = "0.1.0"

[package.metadata.db.entity]
with-serde = "serialize"
compact-format = true
"#;

    const TEST_CARGO_LOCK: &str = r#"
[[package]]
name = "test-app"
version = "0.1.0"
dependencies = [
 "serde",
 "tokio",
]

[[package]]
name = "serde"
version = "1.0.130"

[[package]]
name = "tokio"
version = "1.0.0"
"#;

    fn setup_test_dir(cargo_toml: Option<&str>) -> tree_fs::Tree {
        tree_fs::TreeBuilder::default()
            .add_file("Cargo.toml", cargo_toml.unwrap_or(TEST_CARGO_TOML))
            .create()
            .expect("Failed to create test directory structure")
    }

    fn setup_test_dir_with_lock(
        cargo_toml: Option<&str>,
        cargo_lock: Option<&str>,
    ) -> tree_fs::Tree {
        tree_fs::TreeBuilder::default()
            .add_file("Cargo.toml", cargo_toml.unwrap_or(TEST_CARGO_TOML))
            .add_file("Cargo.lock", cargo_lock.unwrap_or(TEST_CARGO_LOCK))
            .create()
            .expect("Failed to create test directory structure")
    }

    #[test]
    fn test_from_path_valid_toml() {
        let tree = setup_test_dir(None);
        let config = CargoConfig::from_path(tree.root.join("Cargo.toml"))
            .expect("Failed to read Cargo.toml");

        assert_eq!(config.toml["package"]["name"].as_str(), Some("test-app"));
        assert_eq!(config.toml["package"]["version"].as_str(), Some("0.1.0"));
    }

    #[test]
    fn test_from_current_dir() {
        let tree = setup_test_dir(None);
        std::env::set_current_dir(&tree.root).expect("Failed to change directory");

        let config = CargoConfig::from_current_dir().expect("Failed to read from current dir");
        assert_eq!(config.toml["package"]["name"].as_str(), Some("test-app"));
    }

    #[test]
    fn test_lock_from_current_dir() {
        let tree = setup_test_dir_with_lock(None, None);
        std::env::set_current_dir(&tree.root).expect("Failed to change directory");

        let config = CargoConfig::lock_from_current_dir().expect("Failed to read Cargo.lock");
        let packages = config
            .get_package_array()
            .expect("Failed to get package array");
        assert_eq!(packages.len(), 3);
        assert_eq!(
            packages[0].as_table().unwrap()["name"].as_str(),
            Some("test-app")
        );
    }

    #[test]
    fn test_get_package_array() {
        let tree = setup_test_dir_with_lock(None, None);
        let config = CargoConfig::from_path(tree.root.join("Cargo.lock"))
            .expect("Failed to read Cargo.lock");

        let packages = config
            .get_package_array()
            .expect("Failed to get package array");
        assert_eq!(packages.len(), 3);
        assert_eq!(
            packages[1].as_table().unwrap()["name"].as_str(),
            Some("serde")
        );
    }

    #[test]
    fn test_get_db_entities() {
        let tree = setup_test_dir(None);
        let config = CargoConfig::from_path(tree.root.join("Cargo.toml"))
            .expect("Failed to read Cargo.toml");

        let entities = config
            .get_db_entities()
            .expect("No db entities found in Cargo.toml");
        assert_eq!(entities["with-serde"].as_str(), Some("serialize"));
        assert!(entities["compact-format"].as_bool().unwrap());
    }

    #[test]
    fn test_get_db_entities_no_config() {
        let tree = setup_test_dir(Some(
            r#"
[package]
name = "test-app"
version = "0.1.0"
"#,
        ));

        let config = CargoConfig::from_path(tree.root.join("Cargo.toml"))
            .expect("Failed to read Cargo.toml");

        let entities = config.get_db_entities();
        assert!(entities.is_none());
    }

    #[test]
    fn test_invalid_toml() {
        let tree = setup_test_dir(Some(
            r"
[package
invalid toml content
",
        ));

        let result = CargoConfig::from_path(tree.root.join("Cargo.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_file_not_found() {
        let result = CargoConfig::from_path("/non/existent/path/Cargo.toml");
        assert!(result.is_err());
    }
}
