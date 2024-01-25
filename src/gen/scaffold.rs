use std::collections::HashMap;

use lazy_static::lazy_static;
use rrgen::RRgen;
use serde_json::json;

use crate::{app::Hooks, gen};

const CONTROLLER_SCAFFOLD_T: &str = include_str!("templates/controller_scaffold.t");

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
) -> Result<String> {
    // scaffold is never a link table
    let model_messages = model::generate::<H>(rrgen, name, false, fields)?;

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
            columns.push((fname.to_string(), *schema_type));
        }
    }

    let vars = json!({"name": name, "columns": columns, "pkg_name": H::app_name()});
    let res1 = rrgen.generate(CONTROLLER_SCAFFOLD_T, &vars)?;
    let res2 = rrgen.generate(CONTROLLER_TEST_T, &vars)?;
    let messages = collect_messages(vec![res1, res2]);

    Ok(format!("{model_messages}{messages}"))
}
