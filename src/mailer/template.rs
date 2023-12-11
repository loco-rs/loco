//! This module defines a template rendering mechanism for generating email
//! content using Tera templates. It includes functions to read embedded
//! template files, a `Content` struct to hold email content, and a `Template`
//! struct to manage template rendering.
//!
//! # Example
//!
//! ```rust, ignore
//! use include_dir::{include_dir, Dir};
//! use loco_rs::mailer::template::Template;
//!
//! static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
//! let args = serde_json::json!({"name": "framework"});
//! let content = Template::new("contnt").render(&args);
//! ```

use include_dir::Dir;

use crate::{errors::Error, tera, Result};

/// The filename for the subject template file.
const SUBJECT: &str = "subject.t";
/// The filename for the HTML template file.
const HTML: &str = "html.t";
/// The filename for the plain text template file.
const TEXT: &str = "text.t";

/// Reads an embedded file from the provided directory and returns its content
/// as a string.
fn embedded_file(dir: &Dir<'_>, name: &str) -> Result<String> {
    Ok(String::from_utf8_lossy(
        dir.get_file(name)
            .ok_or_else(|| Error::Message(format!("no mailer template file found {name}")))?
            .contents(),
    )
    .to_string())
}

/// A structure representing the content of an email, including subject, text,
/// and HTML.
#[derive(Clone, Debug)]
pub struct Content {
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// A structure for managing template rendering using Tera.
#[derive(Debug, Clone)]
pub struct Template<'a> {
    /// The directory containing the embedded template files.
    dir: &'a Dir<'a>,
}

impl<'a> Template<'a> {
    /// Creates a new `Template` instance with the provided directory.
    pub const fn new(dir: &'a Dir<'_>) -> Self {
        Self { dir }
    }

    /// Renders the email content based on the provided locals using the
    /// embedded templates.
    pub fn render(&self, locals: &serde_json::Value) -> Result<Content> {
        let subject_t = embedded_file(self.dir, SUBJECT)?;
        let text_t = embedded_file(self.dir, TEXT)?;
        let html_t = embedded_file(self.dir, HTML)?;

        // TODO(consider): check+consider offloading to tokio async this work
        let text = tera::render_string(&text_t, locals)?;
        let html = tera::render_string(&html_t, locals)?;
        let subject = tera::render_string(&subject_t, locals)?;
        Ok(Content {
            subject,
            text,
            html,
        })
    }
}

#[cfg(test)]
mod tests {

    use include_dir::include_dir;
    use insta::assert_debug_snapshot;

    use super::*;

    #[test]
    fn can_render_template() {
        let args = serde_json::json!({
            "verifyToken": "1111-2222-3333-4444",
            "name": "Can render test template",
        });
        assert_debug_snapshot!(
            Template::new(&include_dir!("tests/fixtures/email_template/test")).render(&args)
        );
    }
}
