use loco_rs::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub allow_list: Option<Vec<String>>,
}

impl Settings {
    /// Deserialize a strongly typed settings
    ///
    /// # Errors
    ///
    /// This function will return an error if deserialization fails
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(value.clone())?)
    }
}
