use tera::{Context, Kwargs, State, Tera, TeraResult, Value};

use crate::Result;

/// Template name used for the throwaway instance in [`render_string`].
const INLINE_TEMPLATE: &str = "__loco_inline";

/// `get_env(name="VAR", default="fallback")`.
///
/// Tera 1 shipped this as a built-in function; Tera 2 dropped it (only `range`
/// and `throw` remain), so Loco registers its own. The semantics match Tera 1:
/// return the variable when set, fall back to `default` when given, and error
/// when neither is available.
///
/// Every Loco config depends on this — see [`crate::config`].
// `Kwargs` by value is Tera's `Function` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
fn get_env(kwargs: Kwargs, _state: &State<'_>) -> TeraResult<Value> {
    let name: String = kwargs.must_get("name")?;
    match std::env::var(&name) {
        Ok(value) => Ok(Value::from(value)),
        Err(_) => kwargs.get::<Value>("default")?.ok_or_else(|| {
            tera::Error::message(format!(
                "environment variable `{name}` not found and no `default` was given"
            ))
        }),
    }
}

/// Registers the functions Loco guarantees in every rendering context,
/// including the ones Tera 2 no longer provides itself.
fn register_functions(tera: &mut Tera) {
    tera.register_function("get_env", get_env);
}

/// The standard Loco Tera instance, with all built-in filters registered.
/// This is the single factory for every app-facing render path (mailer
/// templates, inline views). Infra rendering that must NOT see app filters
/// (e.g. config/env YAML) uses [`render_string`] instead.
#[must_use]
pub fn instance() -> Tera {
    let mut tera = Tera::default();
    register_functions(&mut tera);
    crate::controller::views::tera_builtins::filters::register_filters(&mut tera);
    tera
}

/// Renders a raw template string WITHOUT app filters. Used for infra rendering
/// such as config/env YAML, where app filters must not apply.
///
/// This cannot use [`Tera::one_off`]: that builds a throwaway instance carrying
/// only Tera's own built-ins, which since Tera 2 no longer include `get_env`.
/// Registering it requires an instance we own.
///
/// # Errors
/// Returns an error if the template fails to render.
pub fn render_string(tera_template: &str, locals: &serde_json::Value) -> Result<String> {
    let mut tera = Tera::default();
    register_functions(&mut tera);
    tera.add_raw_template(INLINE_TEMPLATE, tera_template)?;
    Ok(tera.render(INLINE_TEMPLATE, &Context::from_serialize(locals)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_env_falls_back_to_default_when_unset() {
        let out = render_string(
            r#"{{ get_env(name="LOCO_NOPE_UNSET", default="fallback") }}"#,
            &serde_json::json!({}),
        )
        .unwrap();
        assert_eq!(out, "fallback");
    }

    #[test]
    fn get_env_reads_the_environment() {
        // SAFETY: single-threaded test, variable is unique to this test.
        unsafe { std::env::set_var("LOCO_TERA_TEST_VAR", "from-env") };
        let out = render_string(
            r#"{{ get_env(name="LOCO_TERA_TEST_VAR", default="fallback") }}"#,
            &serde_json::json!({}),
        )
        .unwrap();
        assert_eq!(out, "from-env");
        // SAFETY: as above.
        unsafe { std::env::remove_var("LOCO_TERA_TEST_VAR") };
    }

    #[test]
    fn get_env_errors_when_unset_and_no_default() {
        assert!(render_string(
            r#"{{ get_env(name="LOCO_NOPE_UNSET") }}"#,
            &serde_json::json!({})
        )
        .is_err());
    }
}
