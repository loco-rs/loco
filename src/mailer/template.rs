//! This module defines a template rendering mechanism for generating email
//! content using Tera templates. It includes functions to read embedded
//! template files, a `Content` struct to hold email content, and a `Template`
//! struct to manage template rendering.
//!
//! The mailer template system supports full Tera template features including:
//! - Template inheritance with `{% extends %}` and `{% block %}`
//! - Template includes with `{% include %}`
//! - Shared templates across multiple mailers
//!
//! # Basic Usage
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
//!
//! # Template Inheritance
//!
//! Templates can extend other templates using Tera's inheritance syntax:
//!
//! **base.t** (shared template):
//! ```tera
//! <!DOCTYPE html>
//! <html>
//! <head><title>{% block title %}Email{% endblock %}</title></head>
//! <body>
//!     {% block body %}{% endblock %}
//! </body>
//! </html>
//! ```
//!
//! **html.t** (mailer-specific template):
//! ```tera
//! {% extends "base.t" %}
//! {% block title %}Welcome Email{% endblock %}
//! {% block body %}
//! <h1>Hello {{ name }}!</h1>
//! {% endblock %}
//! ```
//!
//! # Shared Templates Across Mailers
//!
//! Multiple mailers can share common templates (e.g., a base HTML layout):
//!
//! ```rust, ignore
//! use include_dir::{include_dir, Dir};
//! use loco_rs::mailer::template::Template;
//!
//! // Shared base template directory
//! static shared_base: Dir<'_> = include_dir!("src/mailers/shared");
//!
//! // Welcome mailer templates
//! static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
//!
//! // Reset password mailer templates
//! static reset_password: Dir<'_> = include_dir!("src/mailers/auth/reset_password");
//!
//! // Both mailers can use the shared base template
//! let welcome_template = Template::new_with_shared(&welcome, &[&shared_base])?;
//! let reset_template = Template::new_with_shared(&reset_password, &[&shared_base])?;
//! ```
//!
//! Templates from shared directories are loaded first, then templates from the main
//! directory. This means:
//! - Mailer-specific templates can extend templates from shared directories
//! - Mailer-specific templates override any templates with the same name from shared directories
//!
//! # Required Template Files
//!
//! Each mailer template directory must contain three files:
//! - `subject.t` - Email subject line template
//! - `html.t` - HTML email body template
//! - `text.t` - Plain text email body template

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
        Self::new_with_shared(dir, &[])
    }

    /// Creates a new `Template` instance with the provided directory and optional
    /// shared template directories.
    ///
    /// This allows multiple mailers to share common templates (e.g., a base HTML layout).
    /// Templates from shared directories are loaded first, then templates from the main
    /// directory. This means templates in the main directory can extend templates from
    /// shared directories, and templates in the main directory will override any templates
    /// with the same name from shared directories.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// use include_dir::{include_dir, Dir};
    /// use loco_rs::mailer::template::Template;
    ///
    /// // Shared base template directory
    /// static shared_base: Dir<'_> = include_dir!("src/mailers/shared");
    ///
    /// // Welcome mailer templates
    /// static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
    ///
    /// // Create template with shared base templates
    /// let template = Template::new_with_shared(&welcome, &[&shared_base])?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required template files are missing
    /// - Template syntax is invalid
    /// - Building inheritance chains fails
    /// - A template extends a non-existent parent template
    pub fn new_with_shared(dir: &Dir<'_>, shared_dirs: &[&Dir<'_>]) -> Result<Self> {
        let mut tera = Tera::default();

        // First, load templates from shared directories
        // This allows mailer-specific templates to extend shared templates
        for shared_dir in shared_dirs {
            Self::load_templates_from_dir(&mut tera, shared_dir)?;
        }

        // Then, load templates from the main directory
        // These will override any templates with the same name from shared directories
        Self::load_templates_from_dir(&mut tera, dir)?;

        // Build inheritance chains to enable template inheritance, blocks, and extends
        tera.build_inheritance_chains().map_err(|e| {
            Error::Message(format!(
                "failed to build template inheritance chains: {}",
                e
            ))
        })?;

        Ok(Self { tera })
    }

    /// Loads all template files from a directory into the Tera instance.
    ///
    /// Templates are registered by their filename, so templates can reference each other
    /// by simple names (e.g., `{% extends "base.t" %}`).
    fn load_templates_from_dir(tera: &mut Tera, dir: &Dir<'_>) -> Result<()> {
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
        Ok(())
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
    static SHARED_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/shared");
    static WELCOME_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/welcome");
    static RESET_PASSWORD_TEMPLATE_DIR: include_dir::Dir<'_> =
        include_dir!("tests/fixtures/email_template/reset_password");

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
    fn can_render_template_with_inheritance() -> Result<()> {
        let args = serde_json::json!({
            "verifyToken": "ABC-123-XYZ",
            "name": "Test User",
        });
        let template = Template::new(&INHERITANCE_TEMPLATE_DIR)?;
        assert_debug_snapshot!(template.render(&args)?);
        Ok(())
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

    #[test]
    fn can_use_shared_templates_across_mailers() -> Result<()> {
        let args = serde_json::json!({
            "name": "Test User",
        });

        // Welcome mailer using shared base template
        let welcome_template =
            Template::new_with_shared(&WELCOME_TEMPLATE_DIR, &[&SHARED_TEMPLATE_DIR])?;
        assert_debug_snapshot!(welcome_template.render(&args)?);

        // Reset password mailer using the same shared base template
        let reset_args = serde_json::json!({
            "resetUrl": "https://example.com/reset?token=abc123",
        });
        let reset_template =
            Template::new_with_shared(&RESET_PASSWORD_TEMPLATE_DIR, &[&SHARED_TEMPLATE_DIR])?;
        assert_debug_snapshot!(reset_template.render(&reset_args)?);
        Ok(())
    }
}
