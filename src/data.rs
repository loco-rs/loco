use std::{env, path::Path};

use serde::de::DeserializeOwned;

use crate::{Error, Result, env_vars};

const DEFAULT_DATA_FOLDER: &str = "data";
fn data_folder() -> String {
    env::var(env_vars::LOCO_DATA_FOLDER_ENV).unwrap_or_else(|_| DEFAULT_DATA_FOLDER.to_string())
}

/// Load a data JSON file synchronously
///
/// # Errors
///
/// This function will return an error if IO fails
pub fn load_json_file_sync<T: DeserializeOwned>(path: &str) -> Result<T> {
    let p = Path::new(&data_folder()).join(path);
    let content = std::fs::read_to_string(&p).map_err(|e| Error::string(&e.to_string()))?;
    let json_value: T =
        serde_json::from_str(&content).map_err(|e| Error::string(&e.to_string()))?;
    Ok(json_value)
}

/// Load a data JSON file asynchronously
///
/// # Errors
///
/// This function will return an error if IO fails
pub async fn load_json_file<T: DeserializeOwned>(path: &str) -> Result<T> {
    let p = Path::new(&data_folder()).join(path);
    let content = tokio::fs::read_to_string(&p)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;
    let json_value: T =
        serde_json::from_str(&content).map_err(|e| Error::string(&e.to_string()))?;
    Ok(json_value)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use tree_fs::TreeBuilder;

    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_load_json_file_sync_success() {
        // Test successful loading
        let tree = TreeBuilder::default()
            .drop(true)
            .add("test_data.json", r#"{"name": "test", "value": 42}"#)
            .create()
            .expect("Failed to create tree_fs for sync success test");

        // Test with valid JSON
        let file_path = tree.root.join("test_data.json");
        let file_content =
            std::fs::read_to_string(&file_path).expect("Failed to read test_data.json file");
        let result: TestData = serde_json::from_str(&file_content)
            .expect("Failed to parse valid JSON in sync success test");

        assert_eq!(
            result,
            TestData {
                name: "test".to_string(),
                value: 42
            }
        );
    }

    #[test]
    fn test_load_json_file_sync_file_not_found() {
        // Test with non-existent file
        let tree = TreeBuilder::default()
            .drop(true)
            .create()
            .expect("Failed to create tree_fs for sync file not found test");

        let file_path = tree.root.join("nonexistent.json");
        let result = std::fs::read_to_string(file_path);
        result.expect_err("Reading a non-existent file should fail");
    }

    #[test]
    fn test_load_json_file_sync_invalid_json() {
        // Test with invalid JSON
        let tree = TreeBuilder::default()
            .drop(true)
            .add("invalid.json", r#"{"name": "test", "value": not_a_number}"#)
            .create()
            .expect("Failed to create tree_fs for sync invalid JSON test");

        let file_path = tree.root.join("invalid.json");
        let content = std::fs::read_to_string(file_path).expect("Failed to read invalid.json file");
        let result: Result<TestData, _> = serde_json::from_str(&content);
        result.expect_err("Parsing invalid JSON should fail");
    }

    #[tokio::test]
    async fn test_load_json_file_async_success() {
        // Test successful loading
        let tree = TreeBuilder::default()
            .drop(true)
            .add("async_data.json", r#"{"name": "async_test", "value": 100}"#)
            .create()
            .expect("Failed to create tree_fs for async success test");

        // Test with valid JSON
        let file_path = tree.root.join("async_data.json");
        let content = tokio::fs::read_to_string(file_path)
            .await
            .expect("Failed to read async_data.json file");
        let result: TestData = serde_json::from_str(&content)
            .expect("Failed to parse valid JSON in async success test");

        assert_eq!(
            result,
            TestData {
                name: "async_test".to_string(),
                value: 100
            }
        );
    }

    #[tokio::test]
    async fn test_load_json_file_async_file_not_found() {
        // Test with non-existent file
        let tree = TreeBuilder::default()
            .drop(true)
            .create()
            .expect("Failed to create tree_fs for async file not found test");

        let file_path = tree.root.join("nonexistent_async.json");
        let result = tokio::fs::read_to_string(file_path).await;
        result.expect_err("Reading a non-existent file asynchronously should fail");
    }

    #[tokio::test]
    async fn test_load_json_file_async_invalid_json() {
        // Test with invalid JSON
        let tree = TreeBuilder::default()
            .drop(true)
            .add(
                "invalid_async.json",
                r#"{"name": "async_test", "value": not_a_number}"#,
            )
            .create()
            .expect("Failed to create tree_fs for async invalid JSON test");

        let file_path = tree.root.join("invalid_async.json");
        let content = tokio::fs::read_to_string(file_path)
            .await
            .expect("Failed to read invalid_async.json file");
        let result: Result<TestData, _> = serde_json::from_str(&content);
        result.expect_err("Parsing invalid JSON asynchronously should fail");
    }
}
