use std::{borrow::BorrowMut, fs, path::Path};

use serde::Serialize;

use crate::{controller::views::ViewRenderer, Error, Result};

const VIEWS_DIR: &str = "assets/views";

#[derive(Clone, Debug)]
pub struct TeraView {
    #[cfg(debug_assertions)]
    pub tera: std::sync::Arc<std::sync::Mutex<tera::Tera>>,

    #[cfg(not(debug_assertions))]
    pub tera: tera::Tera,

    pub default_context: tera::Context,
}

impl TeraView {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        Self::from_custom_dir(&VIEWS_DIR)
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

        let tera = tera::Tera::new(
            path.as_ref()
                .join("**")
                .join("*.html")
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
        )?;
        let ctx = tera::Context::default();
        Ok(Self {
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
        let context = tera::Context::from_serialize(data)?;

        // NOTE: this supports full reload of template for every render request.
        // it means that you will see refreshed content without rebuild and rerun
        // of the app.
        // the code here is required, since Tera has no "build every time your render"
        // mode, which would have been better.
        // we minimize risk by flagging this in debug (development) builds only
        // for now we leave this commented out, we propose people use `cargo-watch`
        // we want to delay using un__safe as much as possible.
        /*
        #[cfg(debug_assertions)]
        {
            let ptr = std::ptr::addr_of!(self.tera);
            let mut_ptr = ptr.cast_mut();
            // fix this keyword
            un__safe {
                let tera = &mut *mut_ptr;
                tera.full_reload()?;
            }
        }
        */
        #[cfg(debug_assertions)]
        tracing::debug!(key = key, "Tera rendering in non-optimized debug mode");
        #[cfg(debug_assertions)]
        return Ok(self.tera.lock().expect("lock").borrow_mut().render_str(
            &fs::read_to_string(Path::new(VIEWS_DIR).join(key))?,
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
        files:
        - path: template/test.html
          content: |-
            generate test.html file: {{foo}}
        - path: template/test2.html
          content: |-
            generate test2.html file: {{bar}}
        ";

        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();
        let v = TeraView::from_custom_dir(&tree_res).unwrap();

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
