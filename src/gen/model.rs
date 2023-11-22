use cargo_metadata::{MetadataCommand, Package};
use chrono::Utc;
use rrgen::{GenResult, RRgen};
use serde_json::json;

const MODEL_T: &str = include_str!("templates/model.t");
const MODEL_TEST_T: &str = include_str!("templates/model_test.t");

use crate::{errors::Error, Result};

pub fn generate(rrgen: &RRgen, name: &str) -> Result<()> {
    let path = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .current_dir(&path)
        .exec()?;
    let root: &Package = meta
        .root_package()
        .ok_or_else(|| Error::Message("cannot find root package in Cargo.toml".to_string()))?;
    let pkg_name: &str = &root.name;
    let ts = Utc::now();
    let vars = json!({"name": name, "ts": ts, "pkg_name": pkg_name});
    let res1 = rrgen.generate(MODEL_T, &vars)?;
    let res2 = rrgen.generate(MODEL_TEST_T, &vars)?;
    collect_messages(vec![res1, res2]);
    Ok(())
}

fn collect_messages(results: Vec<GenResult>) -> String {
    let mut messages = String::new();
    for res in results {
        if let rrgen::GenResult::Generated {
            message: Some(message),
        } = res
        {
            messages.push_str(&format!("* {message}\n"));
        }
    }
    messages
}
