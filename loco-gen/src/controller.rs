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
    let actions = actions
        .iter()
        .map(|action| json!({"name": action, "verb": verb_for(action)}))
        .collect::<Vec<_>>();
    let vars =
        json!({"name": name, "actions": actions, "auth": auth, "pkg_name": appinfo.app_name});
    r#gen::render_template(rrgen, Path::new("controller/api"), &vars)
}

/// The HTTP method a reader expects an action to answer on.
///
/// Every action used to be wired as `get(..)`, so `loco g controller posts
/// create delete` produced a `create` you could only reach with a GET — and
/// the generated test asserted that same shape, so the defect was held in
/// place by its own coverage.
///
/// The names below are Rails' resource verbs, which is the convention people
/// arrive with. Anything else is a read until the author says otherwise, and
/// GET is both the safe default and trivial to change by hand.
fn verb_for(action: &str) -> &'static str {
    match action {
        "create" => "post",
        "update" => "put",
        "delete" | "destroy" => "delete",
        _ => "get",
    }
}
