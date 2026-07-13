use tera::{Context, Tera};

use crate::Result;

/// The standard Loco Tera instance, with all built-in filters registered.
/// This is the single factory for every app-facing render path (mailer
/// templates, inline views). Infra rendering that must NOT see app filters
/// (e.g. config/env YAML) uses [`render_string`] instead.
#[must_use]
pub fn instance() -> Tera {
    let mut tera = Tera::default();
    crate::controller::views::tera_builtins::filters::register_filters(&mut tera);
    tera
}

/// Renders a raw template string WITHOUT app filters. Used for infra rendering
/// such as config/env YAML, where app filters must not apply.
///
/// # Errors
/// Returns an error if the template fails to render.
pub fn render_string(tera_template: &str, locals: &serde_json::Value) -> Result<String> {
    let text = Tera::one_off(tera_template, &Context::from_serialize(locals)?, false)?;
    Ok(text)
}
