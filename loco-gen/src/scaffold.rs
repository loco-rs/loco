use crate::{
    get_mappings, model, render_template, AppInfo, Error, GenerateResults, Result, ScaffoldKind,
};
use rrgen::RRgen;
use serde_json::json;
use std::path::Path;

pub fn generate(
    rrgen: &RRgen,
    name: &str,
    fields: &[(String, String)],
    kind: &ScaffoldKind,
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    // - scaffold is never a link table
    // - never run with migration_only, because the controllers will refer to the
    //   models. the models only arrive after migration and entities sync.
    let mut gen_result = model::generate(rrgen, name, false, fields, appinfo)?;
    let mappings = get_mappings();

    let mut columns = Vec::new();
    for (fname, ftype) in fields {
        if model::IGNORE_FIELDS.contains(&fname.as_str()) {
            tracing::warn!(
                field = fname,
                "note that a redundant field was specified, it is already generated automatically"
            );
            continue;
        }
        if ftype != "references" && !ftype.starts_with("references:") {
            let schema_type = mappings.rust_field(ftype.as_str()).ok_or_else(|| {
                Error::Message(format!(
                    "type: {} not found. try any of: {:?}",
                    ftype,
                    mappings.rust_fields()
                ))
            })?;
            columns.push((fname.to_string(), schema_type.as_str(), ftype));
        }
    }
    let vars = json!({"name": name, "columns": columns, "pkg_name": appinfo.app_name});
    match kind {
        ScaffoldKind::Api => {
            let res = render_template(rrgen, Path::new("scaffold/api"), &vars)?;
            gen_result.rrgen.extend(res.rrgen);
            gen_result.local_templates.extend(res.local_templates);
        }
        ScaffoldKind::Html => {
            let res = render_template(rrgen, Path::new("scaffold/html"), &vars)?;
            gen_result.rrgen.extend(res.rrgen);
            gen_result.local_templates.extend(res.local_templates);
        }
        ScaffoldKind::Htmx => {
            let res = render_template(rrgen, Path::new("scaffold/htmx"), &vars)?;
            gen_result.rrgen.extend(res.rrgen);
            gen_result.local_templates.extend(res.local_templates);
        }
    }
    Ok(gen_result)
}
