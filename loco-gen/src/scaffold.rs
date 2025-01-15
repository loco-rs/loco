use std::path::Path;

use rrgen::RRgen;
use serde_json::json;

use crate::{
    get_mappings, infer::parse_field_type, model, render_template, AppInfo, Error, GenerateResults,
    Result, ScaffoldKind,
};

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

    let mut columns = Vec::new();
    for (fname, ftype) in fields {
        if model::IGNORE_FIELDS.contains(&fname.as_str()) {
            tracing::warn!(
                field = fname,
                "note that a redundant field was specified, it is already generated automatically"
            );
            continue;
        }

        let field_type = parse_field_type(ftype)?;
        match field_type {
            crate::infer::FieldType::Reference => {
                // (users, "")
                //references.push((fname.to_string(), String::new()));
            }
            crate::infer::FieldType::ReferenceWithCustomField(_refname) => {
                //references.push((fname.to_string(), refname.clone()));
            }
            crate::infer::FieldType::Type(ftype) => {
                let mappings = get_mappings();
                let rust_type = mappings.rust_field(ftype.as_str())?;
                columns.push((fname.to_string(), rust_type.to_string(), ftype));
            }
            crate::infer::FieldType::TypeWithParameters(ftype, params) => {
                let mappings = get_mappings();
                let rust_type = mappings.rust_field_with_params(ftype.as_str(), &params)?;
                let arity = mappings.col_type_arity(ftype.as_str()).unwrap_or_default();
                if params.len() != arity {
                    return Err(Error::Message(format!(
                        "type: `{ftype}` requires specifying {arity} parameters, but only {} were \
                         given (`{}`).",
                        params.len(),
                        params.join(",")
                    )));
                }

                columns.push((fname.to_string(), rust_type.to_string(), ftype));
            }
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
