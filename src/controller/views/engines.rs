use std::path::{Path, PathBuf};

use serde::Serialize;

use super::tera_builtins;
use crate::{controller::views::ViewRenderer, Error, Result};

pub static DEFAULT_ASSET_FOLDER: &str = "assets";

#[derive(Clone, Debug)]
pub struct TeraView {
    #[cfg(debug_assertions)]
    pub tera: std::sync::Arc<std::sync::Mutex<tera::Tera>>,

    #[cfg(not(debug_assertions))]
    pub tera: tera::Tera,

    #[cfg(debug_assertions)]
    pub view_dir: String,

    pub default_context: tera::Context,
}

impl TeraView {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        Self::from_custom_dir(&PathBuf::from(DEFAULT_ASSET_FOLDER).join("views"))
    }

    /// Create a Tera view engine from a custom directory
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn from_custom_dir<P: AsRef<Path>>(path: &P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::string(&format!(
                "missing views directory: `{}`",
                path.as_ref().display()
            )));
        }

        let mut tera = tera::Tera::new(
            path.as_ref()
                .join("**")
                .join("*.html")
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
        )?;
        tera_builtins::filters::register_filters(&mut tera);
        let ctx = tera::Context::default();
        Ok(Self {
            #[cfg(debug_assertions)]
            view_dir: path.as_ref().to_string_lossy().to_string(),
            #[cfg(debug_assertions)]
            tera: std::sync::Arc::new(std::sync::Mutex::new(tera)),
            #[cfg(not(debug_assertions))]
            tera: tera,
            default_context: ctx,
        })
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        #[cfg(debug_assertions)]
        use std::borrow::BorrowMut;

        let context = tera::Context::from_serialize(data)?;

        #[cfg(debug_assertions)]
        tracing::debug!(key = key, "Tera rendering in non-optimized debug mode");
        #[cfg(debug_assertions)]
        return Ok(self.tera.lock().expect("lock").borrow_mut().render_str(
            &std::fs::read_to_string(Path::new(&self.view_dir).join(key))
                .map_err(|_e| tera::Error::template_not_found(key))?,
            &context,
        )?);

        #[cfg(not(debug_assertions))]
        return Ok(self.tera.render(key, &context)?);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tree_fs;

    use super::*;
    #[test]
    fn can_render_view() {
        let yaml_content = r"
        drop: true
        files:
        - path: template/test.html
          content: |-
            generate test.html file: {{foo}}
        - path: template/test2.html
          content: |-
            generate test2.html file: {{bar}}
        ";

        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();
        let v = TeraView::from_custom_dir(&tree_res.root).unwrap();

        assert_eq!(
            v.render("template/test.html", json!({"foo": "foo-txt"}))
                .unwrap(),
            "generate test.html file: foo-txt"
        );

        assert_eq!(
            v.render("template/test2.html", json!({"bar": "bar-txt"}))
                .unwrap(),
            "generate test2.html file: bar-txt"
        );
    }
}
