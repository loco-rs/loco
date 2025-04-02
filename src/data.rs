use std::{env, path::Path};

use serde::de::DeserializeOwned;

use crate::{Error, Result};

const DEFAULT_DATA_FOLDER: &str = "data";
const LOCO_DATA_FOLDER_ENV: &str = "LOCO_DATA";
fn data_folder() -> String {
    env::var(LOCO_DATA_FOLDER_ENV).unwrap_or_else(|_| DEFAULT_DATA_FOLDER.to_string())
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
