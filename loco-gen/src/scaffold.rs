use std::{collections::BTreeSet, path::Path};

use cruet::Inflector;
use heck::ToUpperCamelCase;
use rrgen::RRgen;
use serde_json::{json, Value};

use crate::{
    column::{self, Column, ColumnKind, ScalarType},
    get_mappings,
    infer::parse_field_type,
    model, render_template, AppInfo, Error, GenerateResults, Result, ScaffoldKind,
};

pub fn generate(
    rrgen: &RRgen,
    name: &str,
    with_tz: bool,
    fields: &[(String, String)],
    kind: &ScaffoldKind,
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    // - scaffold is never a link table
    // - never run with migration_only, because the controllers will refer to the
    //   models. the models only arrive after migration and entities sync.
    let mut gen_result = model::generate(rrgen, name, with_tz, fields, appinfo)?;

    match kind {
        ScaffoldKind::Api => {
            // The API scaffold is DTO + controller based (`src/dtos/<plural>.rs` +
            // `src/controllers/<plural>.rs`), a richer shape than the
            // `Params`-struct-based Html/Htmx controllers below, so it gets its
            // own context built straight from `column::Column` (the single
            // source of truth for column type information) rather than the
            // `mappings.json`-based tuples the other two kinds still use.
            // Notably this also means it never runs the `mappings.json`-based
            // loop below, which has no entry for e.g. `enum:..` -- reaching it
            // with an Api-only type would error out before we even get here.
            let api_columns = column::columns_from_fields(fields)?;
            let api_vars = build_api_context(name, &api_columns, with_tz, appinfo);
            let res = render_template(rrgen, Path::new("scaffold/api"), &api_vars)?;
            gen_result.rrgen.extend(res.rrgen);
            gen_result.local_templates.extend(res.local_templates);
        }
        ScaffoldKind::Html | ScaffoldKind::Htmx => {
            let mut columns = Vec::new();
            for (fname, ftype) in fields {
                if model::IGNORE_FIELDS.contains(&fname.as_str()) {
                    tracing::warn!(
                        field = fname,
                        "note that a redundant field was specified, it is already generated \
                         automatically"
                    );
                    continue;
                }

                let field_type = parse_field_type(ftype)?;
                match field_type {
                    crate::infer::FieldType::Reference => {
                        let col_name = format!("{fname}_id");
                        columns.push((col_name, "i32".to_string(), "Integer".to_string()));
                    }
                    crate::infer::FieldType::ReferenceWithCustomField(refname) => {
                        columns.push((refname.clone(), "i32".to_string(), "Integer".to_string()));
                    }
                    crate::infer::FieldType::NullableReference => {
                        let col_name = format!("{fname}_id");
                        columns.push((col_name, "i32".to_string(), "IntegerNull".to_string()));
                    }
                    crate::infer::FieldType::NullableReferenceWithCustomField(refname) => {
                        columns.push((
                            refname.clone(),
                            "i32".to_string(),
                            "IntegerNull".to_string(),
                        ));
                    }
                    crate::infer::FieldType::Type(ftype) => {
                        let mappings = get_mappings();
                        let rust_type = mappings.rust_field(ftype.as_str())?;
                        columns.push((fname.clone(), rust_type.to_string(), ftype));
                    }
                    crate::infer::FieldType::TypeWithParameters(ftype, params) => {
                        let mappings = get_mappings();
                        let rust_type = mappings.rust_field_with_params(ftype.as_str(), &params)?;
                        let arity = mappings.col_type_arity(ftype.as_str()).unwrap_or_default();
                        if params.len() != arity {
                            return Err(Error::Message(format!(
                                "type: `{ftype}` requires specifying {arity} parameters, but only \
                                 {} were given (`{}`).",
                                params.len(),
                                params.join(",")
                            )));
                        }

                        columns.push((fname.clone(), rust_type.to_string(), ftype));
                    }
                }
            }

            let vars = json!({"name": name, "columns": columns, "pkg_name": appinfo.app_name});
            let template_dir = match kind {
                ScaffoldKind::Html => "scaffold/html",
                ScaffoldKind::Htmx => "scaffold/htmx",
                ScaffoldKind::Api => unreachable!("handled in the arm above"),
            };
            let res = render_template(rrgen, Path::new(template_dir), &vars)?;
            gen_result.rrgen.extend(res.rrgen);
            gen_result.local_templates.extend(res.local_templates);
        }
    }
    Ok(gen_result)
}

/// The `sea_orm::prelude` type this column's Rust type needs imported, or
/// `None` when it maps to a primitive/`String`/enum type that needs no
/// prelude import.
const fn prelude_import_for(col: &Column) -> Option<&'static str> {
    let scalar = match &col.kind {
        ColumnKind::Reference { .. } => return None,
        ColumnKind::Array(scalar) | ColumnKind::Scalar(scalar) => scalar,
    };
    match scalar {
        ScalarType::Decimal | ScalarType::DecimalLen { .. } | ScalarType::Money => Some("Decimal"),
        ScalarType::DateTimeTz => Some("DateTimeWithTimeZone"),
        ScalarType::DateTime => Some("DateTime"),
        ScalarType::Date => Some("Date"),
        ScalarType::Time => Some("Time"),
        ScalarType::Uuid => Some("Uuid"),
        _ => None,
    }
}

/// Builds the per-column template context for one user column (`fields` in
/// the DTO/controller templates), plus the enum definition it carries when
/// it's an enum column.
///
/// See the module-level docs on the generator rebuild plan for the exact
/// derivation rules: field naming, resource-prefixed enum types
/// (`{PascalSingular}{UpperCamel(column)}`, e.g. `PostStatus`), and the
/// `From<Model>`/`Set(...)` expressions per column shape.
fn build_field(col: &Column, pascal_singular: &str) -> (Value, Option<Value>) {
    let field_name = match &col.kind {
        ColumnKind::Reference { target, .. } => format!("{target}_id"),
        _ => col.name.clone(),
    };

    if let ColumnKind::Scalar(ScalarType::Enum { values }) = &col.kind {
        let enum_base = col.name.to_singular().to_upper_camel_case();
        let enum_type = format!("{pascal_singular}{enum_base}");

        let variants: Vec<Value> = values
            .iter()
            .map(|v| json!({ "variant": v.to_upper_camel_case(), "value": v }))
            .collect();
        let match_arms: Vec<Value> = values[1..]
            .iter()
            .map(|v| json!({ "variant": v.to_upper_camel_case(), "value": v }))
            .collect();
        let fallback_variant = values[0].to_upper_camel_case();

        let rust_type = if col.nullable {
            format!("Option<{enum_type}>")
        } else {
            enum_type.clone()
        };
        let from_expr = if col.nullable {
            format!("m.{field_name}.map({enum_type}::from)")
        } else {
            format!("{enum_type}::from(m.{field_name})")
        };
        let set_expr = if col.nullable {
            format!("params.{field_name}.map(|v| v.as_str().to_string())")
        } else {
            format!("params.{field_name}.as_str().to_string()")
        };

        let field = json!({
            "field_name": field_name,
            "rust_type": rust_type,
            "ts_override": Value::Null,
            "from_expr": from_expr,
            "set_expr": set_expr,
        });
        let enum_entry = json!({
            "enum_type": enum_type,
            "variants": variants,
            "match_arms": match_arms,
            "fallback_variant": fallback_variant,
        });
        return (field, Some(enum_entry));
    }

    let field = json!({
        "field_name": field_name,
        "rust_type": col.dto_rust_type(),
        "ts_override": col.ts_type(),
        "from_expr": format!("m.{field_name}"),
        "set_expr": format!("params.{field_name}"),
    });
    (field, None)
}

/// Builds the full Tera context for the API scaffold's `dto.t`/`controller.t`
/// templates: resource name forms (`cruet` for plural/singular, `heck` for
/// case -- see `infer.rs`'s inflection note), per-column field data, enum
/// definitions, and the `sea_orm::prelude` import line.
fn build_api_context(
    name: &str,
    columns: &[Column],
    with_tz: bool,
    appinfo: &AppInfo,
) -> Value {
    let singular_raw = name.to_singular();
    let plural_raw = name.to_plural();
    let pascal_singular = singular_raw.to_upper_camel_case();
    // `cruet::Inflector` also has a `to_snake_case`, which would otherwise
    // shadow this call via method resolution -- fully-qualify to guarantee
    // `heck`'s casing (see `infer.rs`'s inflection note: heck owns casing,
    // cruet owns plural/singular only).
    let snake_plural = heck::ToSnakeCase::to_snake_case(plural_raw.as_str());
    let snake_singular = heck::ToSnakeCase::to_snake_case(singular_raw.as_str());

    let mut fields = Vec::new();
    let mut enums = Vec::new();
    let mut prelude_imports: BTreeSet<&'static str> = BTreeSet::new();

    for col in columns {
        if let Some(import) = prelude_import_for(col) {
            prelude_imports.insert(import);
        }
        let (field, enum_entry) = build_field(col, &pascal_singular);
        fields.push(field);
        if let Some(enum_entry) = enum_entry {
            enums.push(enum_entry);
        }
    }

    if with_tz {
        prelude_imports.insert("DateTimeWithTimeZone");
    }

    let prelude_use = if prelude_imports.is_empty() {
        Value::Null
    } else {
        let joined = prelude_imports.into_iter().collect::<Vec<_>>().join(", ");
        Value::String(format!("use sea_orm::prelude::{{{joined}}};"))
    };

    json!({
        "pascal_singular": pascal_singular,
        "snake_plural": snake_plural,
        "snake_singular": snake_singular,
        "pkg_name": appinfo.app_name,
        "with_tz": with_tz,
        "prelude_use": prelude_use,
        "fields": fields,
        "enums": enums,
    })
}
