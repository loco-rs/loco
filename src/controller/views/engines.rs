use std::path::Path;

use serde::Serialize;

use crate::{controller::views::ViewRenderer, Error, Result};

const VIEWS_DIR: &str = "assets/views/";
const VIEWS_GLOB: &str = "assets/views/**/*.html";

#[derive(Clone, Debug)]
pub struct TeraView {
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
        if !Path::new(VIEWS_DIR).exists() {
            return Err(Error::string(&format!(
                "missing views directory: `{VIEWS_DIR}`"
            )));
        }

        let tera = tera::Tera::new(VIEWS_GLOB)?;
        let ctx = tera::Context::default();
        Ok(Self {
            tera,
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

        Ok(self.tera.render(key, &context)?)
    }
}
