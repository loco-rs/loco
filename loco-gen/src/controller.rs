use std::path::Path;

use rrgen::RRgen;
use serde_json::json;

use super::{AppInfo, GenerateResults, Result};
use crate as gen;

pub fn generate(
    rrgen: &RRgen,
    name: &str,
    actions: &[String],
    kind: &gen::ScaffoldKind,
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    let vars = json!({"name": name, "actions": actions, "pkg_name": appinfo.app_name});
    match kind {
        gen::ScaffoldKind::Api => gen::render_template(rrgen, Path::new("controller/api"), &vars),
        gen::ScaffoldKind::Html => {
            let mut gen_result =
                gen::render_template(rrgen, Path::new("controller/html/controller.t"), &vars)?;
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": appinfo.app_name});
                let res = gen::render_template(rrgen, Path::new("controller/html/view.t"), &vars)?;
                gen_result.rrgen.extend(res.rrgen);
                gen_result.local_templates.extend(res.local_templates);
            }
            Ok(gen_result)
        }
        gen::ScaffoldKind::Htmx => {
            let mut gen_result =
                gen::render_template(rrgen, Path::new("controller/htmx/controller.t"), &vars)?;
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": appinfo.app_name});
                let res = gen::render_template(rrgen, Path::new("controller/htmx/view.t"), &vars)?;
                gen_result.rrgen.extend(res.rrgen);
                gen_result.local_templates.extend(res.local_templates);
            }
            Ok(gen_result)
        }
    }
}
