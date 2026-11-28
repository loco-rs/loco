use super::{AppInfo, GenerateResults, Result};
use crate as r#gen;
use crate::RRgen;
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
    let actions = handler_actions(actions)
        .into_iter()
        .map(|action| json!({"name": action, "verb": verb_for(action)}))
        .collect::<Vec<_>>();
    let vars =
        json!({"name": name, "actions": actions, "auth": auth, "pkg_name": appinfo.app_name});
    r#gen::render_template(rrgen, Path::new("controller/api"), &vars)
}

/// The actions that get a handler of their own, in the order given.
///
/// The template always writes `index` (wired to the controller's root, `/`),
/// so `index` among the actions would be a second `fn index` and the app would
/// stop compiling ("the name `index` is defined multiple times"). `index` is
/// the first action a Rails user types (`rails g controller posts index show`),
/// so it is dropped here rather than rejected. A repeated action would be a
/// duplicate function the same way, so repeats are dropped too.
fn handler_actions(actions: &[String]) -> Vec<&str> {
    let mut seen = Vec::new();
    for action in actions {
        let action = action.as_str();
        if action != "index" && !seen.contains(&action) {
            seen.push(action);
        }
    }
    seen
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
