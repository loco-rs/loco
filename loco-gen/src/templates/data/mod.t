{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/data/{{module_name}}.rs"
skip_exists: true
message: "Data loader `{{struct_name}}` was added successfully."
injections:
- into: "src/data/mod.rs"
  append: true
  content: "pub mod {{ module_name }};"
---
use loco_rs::{data, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const DATA_FILE: &str = "{{module_name}}/data.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct {{struct_name}} {
    pub is_loaded: bool,
}

#[allow(dead_code)]
/// Reads the data from the JSON file.
///
/// # Errors
/// This function returns an error if the file cannot be read or deserialized.
pub async fn read() -> Result<{{struct_name}}> {
    data::load_json_file(DATA_FILE).await
}

static DATA: OnceLock<{{struct_name}}> = OnceLock::new();
#[allow(dead_code)]
pub fn get() -> &'static {{struct_name}} {
    DATA.get_or_init(|| data::load_json_file_sync(DATA_FILE).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access() {
        assert!(&get().is_loaded);
    }
}
