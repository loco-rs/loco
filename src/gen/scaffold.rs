use std::collections::HashMap;

use lazy_static::lazy_static;
use rrgen::RRgen;
use serde_json::json;

use crate::{app::Hooks, gen};

const API_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/api/controller.t");

const HTMX_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/controller.t");
const HTMX_VIEW_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view.t");
const HTMX_VIEW_EDIT_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_edit.t");
const HTMX_VIEW_CREATE_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_create.t");
const HTMX_VIEW_SHOW_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_show.t");
const HTMX_VIEW_LIST_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_list.t");

const HTML_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/controller.t");
const HTML_VIEW_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view.t");
const HTML_VIEW_EDIT_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_edit.t");
const HTML_VIEW_CREATE_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_create.t");
const HTML_VIEW_SHOW_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_show.t");
const HTML_VIEW_LIST_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_list.t");

use super::{collect_messages, model, CONTROLLER_TEST_T};
use crate::{errors::Error, Result};

lazy_static! {
    static ref PARAMS_MAPPING: HashMap<&'static str, &'static str> = HashMap::from([
        ("text", "Option<String>"),
        ("string", "Option<String>"),
        ("string!", "String"),
        ("string^", "String"),
        ("int", "Option<i32>"),
        ("int!", "i32"),
        ("int^", "Option<i32>"),
        ("bool", "Option<bool>"),
        ("bool!", "bool"),
        ("ts", "Option<DateTime>"),
        ("ts!", "DateTime"),
        ("uuid", "Option<Uuid>"),
        ("uuid!", "Uuid"),
        ("json", "Option<serde_json::Value>"),
        ("json!", "serde_json::Value"),
        ("jsonb", "Option<serde_json::Value>"),
        ("jsonb!", "serde_json::Value"),
    ]);
}

pub fn generate<H: Hooks>(
    rrgen: &RRgen,
    name: &str,
    fields: &[(String, String)],
    kind: &gen::ScaffoldKind,
) -> Result<String> {
    // - scaffold is never a link table
    // - never run with migration_only, because the controllers will refer to the
    //   models. the models only arrive after migration and entities sync.
    let model_messages = model::generate::<H>(rrgen, name, false, false, fields)?;

    let mut columns = Vec::new();
    for (fname, ftype) in fields {
        if gen::model::IGNORE_FIELDS.contains(&fname.as_str()) {
            tracing::warn!(
                field = fname,
                "note that a redundant field was specified, it is already generated automatically"
            );
            continue;
        }
        if ftype != "references" {
            let schema_type = PARAMS_MAPPING.get(ftype.as_str()).ok_or_else(|| {
                Error::Message(format!(
                    "type: {} not found. try any of: {:?}",
                    ftype,
                    PARAMS_MAPPING.keys()
                ))
            })?;
            columns.push((fname.to_string(), *schema_type, ftype));
        }
    }
    let vars = json!({"name": name, "columns": columns, "pkg_name": H::app_name()});
    match kind {
        gen::ScaffoldKind::Api => {
            let res1 = rrgen.generate(API_CONTROLLER_SCAFFOLD_T, &vars)?;
            let res2 = rrgen.generate(CONTROLLER_TEST_T, &vars)?;
            let messages = collect_messages(vec![res1, res2]);
            Ok(format!("{model_messages}{messages}"))
        }
        gen::ScaffoldKind::Html => {
            rrgen.generate(HTML_CONTROLLER_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_EDIT_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_CREATE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_SHOW_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_LIST_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_SCAFFOLD_T, &vars)?;
            Ok(model_messages)
        }
        gen::ScaffoldKind::Htmx => {
            rrgen.generate(HTMX_CONTROLLER_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_EDIT_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_CREATE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_SHOW_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_LIST_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_SCAFFOLD_T, &vars)?;
            Ok(model_messages)
        }
    }
}
