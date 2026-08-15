use super::{AppInfo, GenerateResults, Result};
use crate as r#gen;
use rrgen::RRgen;
use serde_json::json;
use std::path::Path;

/// Loco 1.0 ships a single controller flavor (JSON API). `actions` names the
/// handlers to stub out in the generated controller.
///
/// `auth` adds an `auth::JWT` extractor to every generated handler. A bare
/// controller is public by default — it has nothing to protect until you write
/// the body — so this is opt-in (`--auth`), the mirror of the scaffold's
/// opt-out `--no-auth`.
pub fn generate(
    rrgen: &RRgen,
    name: &str,
    actions: &[String],
    auth: bool,
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    let vars =
        json!({"name": name, "actions": actions, "auth": auth, "pkg_name": appinfo.app_name});
    r#gen::render_template(rrgen, Path::new("controller/api"), &vars)
}
