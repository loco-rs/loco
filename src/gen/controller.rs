use rrgen::RRgen;
use serde_json::json;

use crate::{
    app::{AppContextTrait, Hooks},
    gen,
};

const API_CONTROLLER_CONTROLLER_T: &str = include_str!("templates/controller/api/controller.t");
const API_CONTROLLER_TEST_T: &str = include_str!("templates/controller/api/test.t");

const HTMX_CONTROLLER_CONTROLLER_T: &str = include_str!("templates/controller/htmx/controller.t");
const HTMX_VIEW_T: &str = include_str!("templates/controller/htmx/view.t");

const HTML_CONTROLLER_CONTROLLER_T: &str = include_str!("templates/controller/html/controller.t");
const HTML_VIEW_T: &str = include_str!("templates/controller/html/view.t");

use super::collect_messages;
use crate::Result;

pub fn generate<AC: AppContextTrait, H: Hooks<AC>>(
    rrgen: &RRgen,
    name: &str,
    actions: &[String],
    kind: &gen::ScaffoldKind,
) -> Result<String> {
    let vars = json!({"name": name, "actions": actions, "pkg_name": H::app_name()});
    match kind {
        gen::ScaffoldKind::Api => {
            let res1 = rrgen.generate(API_CONTROLLER_CONTROLLER_T, &vars)?;
            let res2 = rrgen.generate(API_CONTROLLER_TEST_T, &vars)?;
            let messages = collect_messages(vec![res1, res2]);
            Ok(messages)
        }
        gen::ScaffoldKind::Html => {
            let mut messages = Vec::new();
            let res = rrgen.generate(HTML_CONTROLLER_CONTROLLER_T, &vars)?;
            messages.push(res);
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": H::app_name()});
                messages.push(rrgen.generate(HTML_VIEW_T, &vars)?);
            }
            Ok(collect_messages(messages))
        }
        gen::ScaffoldKind::Htmx => {
            let mut messages = Vec::new();
            let res = rrgen.generate(HTMX_CONTROLLER_CONTROLLER_T, &vars)?;
            messages.push(res);
            for action in actions {
                let vars = json!({"name": name, "action": action, "pkg_name": H::app_name()});
                messages.push(rrgen.generate(HTMX_VIEW_T, &vars)?);
            }
            Ok(collect_messages(messages))
        }
    }
}
