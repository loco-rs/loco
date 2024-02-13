use tera::{Context, Tera};

use crate::Result;

pub fn render_string(tera_template: &str, locals: &serde_json::Value) -> Result<String> {
    let text = Tera::one_off(tera_template, &Context::from_serialize(locals)?, false)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_string() {
        let template = "Loco, {{ name }}";
        let locals = serde_json::json!({"name": "website"});

        let result = render_string(template, &locals).unwrap();

        assert_eq!(result, "Loco, website");
    }
}
