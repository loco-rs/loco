use super::{AppInfo, GenerateResults, Result};
use crate as r#gen;
use rrgen::RRgen;
use serde_json::json;
use std::path::Path;

/// Loco 1.0 ships a single controller flavor (JSON API). `actions` names the
/// handlers to stub out in the generated controller.
pub fn generate(
    rrgen: &RRgen,
    name: &str,
    actions: &[String],
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    let vars = json!({"name": name, "actions": actions, "pkg_name": appinfo.app_name});
    r#gen::render_template(rrgen, Path::new("controller/api"), &vars)
}
