use super::{AppInfo, Result};
use crate as gen;
use rrgen::{GenResult, RRgen};
use serde_json::json;
use std::path::Path;

pub fn generate(
    rrgen: &RRgen,
    name: &str,
    actions: &[String],
    kind: &gen::ScaffoldKind,
    appinfo: &AppInfo,
) -> Result<Vec<GenResult>> {
    let vars = json!({"name": name, "actions": actions, "pkg_name": appinfo.app_name});
    match kind {
        gen::ScaffoldKind::Api => gen::render_template(rrgen, Path::new("controller/api"), &vars),
        gen::ScaffoldKind::Html => {
            let mut gen_result =
                gen::render_template(rrgen, Path::new("controller/html/controller.t"), &vars)?;
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": appinfo.app_name});
                gen_result.extend(gen::render_template(
                    rrgen,
                    Path::new("controller/html/view.t"),
                    &vars,
                )?);
            }
            Ok(gen_result)
        }
        gen::ScaffoldKind::Htmx => {
            let mut gen_result =
                gen::render_template(rrgen, Path::new("controller/htmx/controller.t"), &vars)?;
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": appinfo.app_name});
                gen_result.extend(gen::render_template(
                    rrgen,
                    Path::new("controller/htmx/view.t"),
                    &vars,
                )?);
            }
            Ok(gen_result)
        }
    }
}
