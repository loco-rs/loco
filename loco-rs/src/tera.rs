use tera::{Context, Tera};

use crate::Result;

pub fn render_string(tera_template: &str, locals: &serde_json::Value) -> Result<String> {
    let text = Tera::one_off(tera_template, &Context::from_serialize(locals)?, false)?;
    Ok(text)
}
