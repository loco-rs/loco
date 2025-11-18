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
//! let template = Template::new(&welcome)?;
//! let content = template.render(&args)?;
//! ```

use include_dir::Dir;
use tera::{Context, Tera};

use crate::{errors::Error, Result};

/// The filename for the subject template file.
const SUBJECT: &str = "subject.t";
/// The filename for the HTML template file.
const HTML: &str = "html.t";
/// The filename for the plain text template file.
const TEXT: &str = "text.t";

/// A structure representing the content of an email, including subject, text,
/// and HTML.
#[derive(Clone, Debug)]
pub struct Content {
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// A structure for managing template rendering using Tera.
/// This properly initializes Tera to support template inheritance, blocks, and extends.
#[derive(Debug)]
pub struct Template {
    /// The Tera instance with all templates from the directory loaded.
    tera: Tera,
}

impl Template {
    /// Creates a new `Template` instance with the provided directory.
    /// This initializes a Tera instance with all templates from the directory,
    /// enabling template inheritance, blocks, and extends.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required template files are missing
    /// - Template syntax is invalid
    /// - Building inheritance chains fails
    pub fn new(dir: &Dir<'_>) -> Result<Self> {
        let mut tera = Tera::default();

        // Load all template files from the directory into Tera
        // Use the filename as the template name to ensure consistent naming
        for entry in dir.files() {
            let path = entry.path();
            // Use the filename (last component) as the template name
            // This ensures templates can reference each other by simple names
            let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                Error::Message(format!("invalid template path: {}", path.to_string_lossy()))
            })?;
            let content = String::from_utf8_lossy(entry.contents()).to_string();
            tera.add_raw_template(name, &content)
                .map_err(|e| Error::Message(format!("failed to add template '{}': {}", name, e)))?;
        }

        // Build inheritance chains to enable template inheritance, blocks, and extends
        tera.build_inheritance_chains().map_err(|e| {
            Error::Message(format!(
                "failed to build template inheritance chains: {}",
                e
            ))
        })?;

        Ok(Self { tera })
    }

    /// Renders the email content based on the provided locals using the
    /// embedded templates.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required template files are missing
    /// - Template rendering fails
    pub fn render(&self, locals: &serde_json::Value) -> Result<Content> {
        let context = Context::from_serialize(locals)
            .map_err(|e| Error::Message(format!("failed to create template context: {}", e)))?;

        let subject = self
            .tera
            .render(SUBJECT, &context)
            .map_err(|e| Error::Message(format!("failed to render subject template: {}", e)))?;

        let text = self
            .tera
            .render(TEXT, &context)
            .map_err(|e| Error::Message(format!("failed to render text template: {}", e)))?;

        let html = self
            .tera
            .render(HTML, &context)
            .map_err(|e| Error::Message(format!("failed to render html template: {}", e)))?;

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

    static TEST_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/test");
    static INHERITANCE_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/inheritance");
    static INVALID_INHERITANCE_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/invalid_inheritance");

    #[test]
    fn can_render_template() {
        let args = serde_json::json!({
            "verifyToken": "1111-2222-3333-4444",
            "name": "Can render test template",
        });
        let template = Template::new(&TEST_TEMPLATE_DIR).unwrap();
        assert_debug_snapshot!(template.render(&args));
    }

    #[test]
    fn can_render_template_with_inheritance() {
        let args = serde_json::json!({
            "verifyToken": "ABC-123-XYZ",
            "name": "Test User",
        });
        let template = Template::new(&INHERITANCE_TEMPLATE_DIR).unwrap();
        let content = template.render(&args).unwrap();

        // Verify that template inheritance worked
        // Subject should be rendered directly (no inheritance)
        assert_eq!(content.subject.trim(), "Welcome Test User!");

        // HTML should extend base.t and include the full HTML structure
        assert!(content.html.contains("<html>"));
        assert!(content.html.contains("<head>"));
        assert!(content.html.contains("<title>Welcome Email</title>"));
        assert!(content.html.contains("<h1>Hello Test User!</h1>"));
        assert!(content.html.contains("ABC-123-XYZ"));
        // Check for closing tags separately (they might be on different lines)
        assert!(content.html.contains("</body>"));
        assert!(content.html.contains("</html>"));

        // Text should be rendered directly (no inheritance)
        assert!(content.text.contains("Hello Test User!"));
        assert!(content.text.contains("ABC-123-XYZ"));
        assert!(content.text.contains("Thank you for using our service"));
    }

    #[test]
    fn fails_when_extending_nonexistent_template() {
        // Attempting to create a template with a reference to a non-existent parent
        // should fail during initialization when building inheritance chains
        let result = Template::new(&INVALID_INHERITANCE_TEMPLATE_DIR);

        assert!(
            result.is_err(),
            "Template::new should fail when extending non-existent template"
        );

        let error = result.unwrap_err();
        let error_msg = error.to_string();

        // Verify the error message mentions the missing template or inheritance issue
        assert!(
            error_msg.contains("non_existent_base.t")
                || error_msg.contains("inheritance")
                || error_msg.contains("not found")
                || error_msg.contains("missing"),
            "Error message should mention the missing template or inheritance issue. Got: {}",
            error_msg
        );
    }
}
