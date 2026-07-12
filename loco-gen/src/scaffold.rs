use std::{collections::BTreeSet, path::Path};

use cruet::Inflector;
use heck::{ToLowerCamelCase, ToUpperCamelCase};
use rrgen::RRgen;
use serde_json::{json, Value};

use crate::{
    column::{self, Column, ColumnKind, ScalarType},
    model, render_template, AppInfo, GenerateResults, Result,
};

/// Loco 1.0 ships a single scaffold flavor built straight from
/// `column::Column` (the single source of truth for column type
/// information) via [`build_api_context`]. It is **adaptive**: the typed
/// backend (`src/dtos/<plural>.rs` + `src/controllers/<plural>.rs`) is always
/// emitted, so a headless/REST app gets a typed resource; the React-SPA
/// frontend (`frontend/src/api/<plural>.ts` + `frontend/src/pages/<plural>/
/// *.tsx` + the `routes.tsx` injection) is emitted only when `frontend` is
/// `true` -- i.e. the app has a clientside `frontend/` -- so non-SPA apps get
/// no orphan frontend files.
pub fn generate(
    rrgen: &RRgen,
    name: &str,
    with_tz: bool,
    fields: &[(String, String)],
    frontend: bool,
    appinfo: &AppInfo,
) -> Result<GenerateResults> {
    // - scaffold is never a link table
    // - never run with migration_only, because the controllers will refer to the
    //   models. the models only arrive after migration and entities sync.
    let mut gen_result = model::generate(rrgen, name, with_tz, fields, appinfo)?;

    let api_columns = column::columns_from_fields(fields)?;
    let api_vars = build_api_context(name, &api_columns, with_tz, appinfo);

    // Backend (DTO + controller) -- always emitted.
    let res = render_template(rrgen, Path::new("scaffold/api"), &api_vars)?;
    gen_result.rrgen.extend(res.rrgen);
    gen_result.local_templates.extend(res.local_templates);

    // Frontend (React Query hooks + pages + routes injection) -- only when the
    // app has a clientside frontend.
    if frontend {
        let fres = render_template(rrgen, Path::new("scaffold/frontend"), &api_vars)?;
        gen_result.rrgen.extend(fres.rrgen);
        gen_result.local_templates.extend(fres.local_templates);
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

/// Human-friendly label for a column, used by the frontend List/New/Edit/Show
/// templates, e.g. `published_at` -> `Published at` (only the leading
/// character is capitalized -- matches `examples/reference_spa`'s
/// hand-written labels such as "Created at").
fn humanize_label(field_name: &str) -> String {
    let mut label = field_name.replace('_', " ");
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}

/// The HTML form control the frontend New/Edit templates should render for
/// this column. Finer-grained than `column::FormKind`: splits `Date` from
/// `Time`/`DateTime`/`DateTimeTz` (only a bare `Date` column gets
/// `type="date"`, the rest get `type="datetime-local"`), and splits
/// `Decimal`/`DecimalLen`/`Money` -- which round-trip through TS as plain
/// strings, so the reference SPA edits them as text, not `type="number"` --
/// from true numeric types.
fn frontend_input_kind(col: &Column) -> &'static str {
    match &col.kind {
        ColumnKind::Reference { .. } => "number",
        ColumnKind::Array(_) => "textarea",
        ColumnKind::Scalar(scalar) => match scalar {
            ScalarType::Enum { .. } => "select",
            ScalarType::Bool => "checkbox",
            ScalarType::Date => "date",
            ScalarType::Time | ScalarType::DateTime | ScalarType::DateTimeTz => "datetime",
            ScalarType::Decimal | ScalarType::DecimalLen { .. } | ScalarType::Money => {
                "text_number"
            }
            ScalarType::Text
            | ScalarType::Json
            | ScalarType::Jsonb
            | ScalarType::Blob
            | ScalarType::VarBinary { .. }
            | ScalarType::BinaryLen { .. } => "textarea",
            ScalarType::SmallInt
            | ScalarType::Int
            | ScalarType::BigInt
            | ScalarType::SmallUnsigned
            | ScalarType::Unsigned
            | ScalarType::BigUnsigned
            | ScalarType::Float
            | ScalarType::Double => "number",
            ScalarType::String | ScalarType::Uuid => "text",
        },
    }
}

/// The `New.tsx` initial-state JS literal for a non-enum column: `bool` gets
/// `false`, any nullable column gets `null`, numeric-input columns get `0`,
/// everything else (text/textarea/text_number/date/datetime) gets `""`. Enum
/// columns are handled separately by the caller (first enum value wins,
/// ahead of this nullable/number/string fallback chain).
fn frontend_initial_value(col: &Column, input_kind: &str) -> String {
    if matches!(&col.kind, ColumnKind::Scalar(ScalarType::Bool)) {
        "false".to_string()
    } else if col.nullable {
        "null".to_string()
    } else if input_kind == "number" {
        "0".to_string()
    } else {
        "\"\"".to_string()
    }
}

/// Builds the per-column template context for one user column (`fields` in
/// the DTO/controller/frontend templates), plus the enum definition it
/// carries when it's an enum column.
///
/// See the module-level docs on the generator rebuild plan for the exact
/// derivation rules: field naming, resource-prefixed enum types
/// (`{PascalSingular}{UpperCamel(column)}`, e.g. `PostStatus`), and the
/// `From<Model>`/`Set(...)` expressions per column shape. The `label`/
/// `is_enum`/`enum_type`/`options_const`/`nullable`/`input_kind`/
/// `initial_value` keys are frontend-only additions consumed by the
/// `frontend_*.t` templates (`loco-gen/src/templates/scaffold/api/`).
fn build_field(col: &Column, pascal_singular: &str) -> (Value, Option<Value>) {
    let field_name = match &col.kind {
        ColumnKind::Reference { target, .. } => format!("{target}_id"),
        _ => col.name.clone(),
    };
    let label = humanize_label(&field_name);

    if let ColumnKind::Scalar(ScalarType::Enum { values }) = &col.kind {
        let enum_base = col.name.to_singular().to_upper_camel_case();
        let enum_type = format!("{pascal_singular}{enum_base}");
        let options_const = format!("{}_OPTIONS", col.name.to_uppercase());

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
            "label": label,
            "nullable": col.nullable,
            "is_enum": true,
            "enum_type": enum_type,
            "options_const": options_const,
            "input_kind": "select",
            "initial_value": format!("\"{}\"", values[0]),
        });
        let enum_entry = json!({
            "enum_type": enum_type,
            "variants": variants,
            "match_arms": match_arms,
            "fallback_variant": fallback_variant,
            "options_const": options_const,
        });
        return (field, Some(enum_entry));
    }

    let input_kind = frontend_input_kind(col);
    let initial_value = frontend_initial_value(col, input_kind);

    let field = json!({
        "field_name": field_name,
        "rust_type": col.dto_rust_type(),
        "ts_override": col.ts_type(),
        "from_expr": format!("m.{field_name}"),
        "set_expr": format!("params.{field_name}"),
        "label": label,
        "nullable": col.nullable,
        "is_enum": false,
        "enum_type": Value::Null,
        "options_const": Value::Null,
        "input_kind": input_kind,
        "initial_value": initial_value,
    });
    (field, None)
}

/// Builds the full Tera context for the API scaffold's `dto.t`/`controller.t`
/// templates: resource name forms (`cruet` for plural/singular, `heck` for
/// case -- see `infer.rs`'s inflection note), per-column field data, enum
/// definitions, and the `sea_orm::prelude` import line.
fn build_api_context(name: &str, columns: &[Column], with_tz: bool, appinfo: &AppInfo) -> Value {
    let singular_raw = name.to_singular();
    let plural_raw = name.to_plural();
    let pascal_singular = singular_raw.to_upper_camel_case();
    // `cruet::Inflector` also has a `to_snake_case`, which would otherwise
    // shadow this call via method resolution -- fully-qualify to guarantee
    // `heck`'s casing (see `infer.rs`'s inflection note: heck owns casing,
    // cruet owns plural/singular only).
    let snake_plural = heck::ToSnakeCase::to_snake_case(plural_raw.as_str());
    let snake_singular = heck::ToSnakeCase::to_snake_case(singular_raw.as_str());
    // Frontend-only resource-name forms: `pascal_plural` for
    // `ListPostsParams`/`useListPosts`/the `List.tsx` `<h1>`, `camel_singular`
    // for the `postKeys` query-key object and the `List.tsx` row-lambda
    // binding.
    let pascal_plural = plural_raw.to_upper_camel_case();
    let camel_singular = singular_raw.to_lower_camel_case();

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

    // `Show.tsx`'s `<h1>` picks the first bare (non-`text`, non-nullable-vs-
    // nullable-agnostic) `string` column as the "title" -- e.g. `title` in
    // the reference `post` resource -- and the `<dl>` below it renders every
    // *other* field. Falls back to the first column at all when no bare
    // `string` column exists, so the template always has something to head
    // the page with.
    let title_field_name = columns
        .iter()
        .find(|c| matches!(c.kind, ColumnKind::Scalar(ScalarType::String)))
        .or_else(|| columns.first())
        .map(|c| c.name.clone());

    // Route-injection payloads for `frontend/src/routes.tsx` (see
    // `frontend_list.t`'s `injections`): built here, rather than looped in
    // the template, because YAML block-scalar indentation inside a Tera
    // frontmatter is fragile -- a single quoted flow scalar with `\n`
    // (literal backslash-n, decoded to a real newline by `serde_yaml`) per
    // line sidesteps that entirely. `frontend/src/routes.tsx` must carry the
    // `// scaffold:imports` / `// scaffold:routes` anchor comments (a 2c
    // dependency on the once-per-app base `routes.tsx`).
    let frontend_imports_injection = ["Edit", "List", "New", "Show"]
        .iter()
        .map(|component| {
            format!("import {{ {component} }} from './pages/{snake_plural}/{component}'")
        })
        .collect::<Vec<_>>()
        .join("\\n");
    let frontend_routes_injection = [
        (String::new(), "List"),
        ("/new".to_string(), "New"),
        ("/:id".to_string(), "Show"),
        ("/:id/edit".to_string(), "Edit"),
    ]
    .iter()
    .map(|(suffix, component)| {
        format!("          {{ path: '{snake_plural}{suffix}', element: <{component} /> }},")
    })
    .collect::<Vec<_>>()
    .join("\\n");

    json!({
        "pascal_singular": pascal_singular,
        "pascal_plural": pascal_plural,
        "camel_singular": camel_singular,
        "snake_plural": snake_plural,
        "snake_singular": snake_singular,
        "pkg_name": appinfo.app_name,
        "with_tz": with_tz,
        "prelude_use": prelude_use,
        "fields": fields,
        "enums": enums,
        "title_field_name": title_field_name,
        "frontend_imports_injection": frontend_imports_injection,
        "frontend_routes_injection": frontend_routes_injection,
    })
}
