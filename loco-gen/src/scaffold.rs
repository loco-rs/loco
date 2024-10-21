use rrgen::RRgen;
use serde_json::json;

use crate as gen;

const API_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/api/controller.t");
const API_CONTROLLER_TEST_T: &str = include_str!("templates/scaffold/api/test.t");

const HTMX_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/controller.t");
const HTMX_BASE_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/base.t");
const HTMX_VIEW_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view.t");
const HTMX_VIEW_EDIT_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_edit.t");
const HTMX_VIEW_CREATE_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_create.t");
const HTMX_VIEW_SHOW_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_show.t");
const HTMX_VIEW_LIST_SCAFFOLD_T: &str = include_str!("templates/scaffold/htmx/view_list.t");

const HTML_CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/controller.t");
const HTML_BASE_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/base.t");
const HTML_VIEW_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view.t");
const HTML_VIEW_EDIT_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_edit.t");
const HTML_VIEW_CREATE_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_create.t");
const HTML_VIEW_SHOW_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_show.t");
const HTML_VIEW_LIST_SCAFFOLD_T: &str = include_str!("templates/scaffold/html/view_list.t");

use super::{collect_messages, model, AppInfo, Error, Result, MAPPINGS};

pub fn generate(
    rrgen: &RRgen,
    name: &str,
    fields: &[(String, String)],
    kind: &gen::ScaffoldKind,
    appinfo: &AppInfo,
) -> Result<String> {
    // - scaffold is never a link table
    // - never run with migration_only, because the controllers will refer to the
    //   models. the models only arrive after migration and entities sync.
    let model_messages = model::generate(rrgen, name, false, false, fields, appinfo)?;

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
            let schema_type = MAPPINGS.rust_field(ftype.as_str()).ok_or_else(|| {
                Error::Message(format!(
                    "type: {} not found. try any of: {:?}",
                    ftype,
                    MAPPINGS.rust_fields()
                ))
            })?;
            columns.push((fname.to_string(), schema_type.as_str(), ftype));
        }
    }
    let vars = json!({"name": name, "columns": columns, "pkg_name": appinfo.app_name});
    match kind {
        gen::ScaffoldKind::Api => {
            let res1 = rrgen.generate(API_CONTROLLER_SCAFFOLD_T, &vars)?;
            let res2 = rrgen.generate(API_CONTROLLER_TEST_T, &vars)?;
            let messages = collect_messages(vec![res1, res2]);
            Ok(format!("{model_messages}{messages}"))
        }
        gen::ScaffoldKind::Html => {
            rrgen.generate(HTML_CONTROLLER_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_BASE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_EDIT_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_CREATE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_SHOW_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_LIST_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTML_VIEW_SCAFFOLD_T, &vars)?;
            Ok(model_messages)
        }
        gen::ScaffoldKind::Htmx => {
            rrgen.generate(HTMX_CONTROLLER_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_BASE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_EDIT_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_CREATE_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_SHOW_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_LIST_SCAFFOLD_T, &vars)?;
            rrgen.generate(HTMX_VIEW_SCAFFOLD_T, &vars)?;
            Ok(model_messages)
        }
    }
}
