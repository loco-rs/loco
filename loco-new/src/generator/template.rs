//! This module defines a `Template` struct for handling template files.
//! It includes methods to identify template files, render templates
//! with injected settings, and modify file paths by stripping specific extensions.

use crate::settings::Settings;
use rand::{distributions::Alphanumeric, rngs::StdRng, Rng, SeedableRng};
use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};

const TEMPLATE_EXTENSION: &str = "t";

fn generate_random_string<R: Rng>(rng: &mut R, length: u64) -> String {
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

/// Represents a template that can be rendered with injected settings.
#[derive(Debug, Clone)]
pub struct Template {
    rng: Arc<Mutex<StdRng>>,
}

impl Default for Template {
    fn default() -> Self {
        #[cfg(test)]
        let rng = StdRng::seed_from_u64(42);
        #[cfg(not(test))]
        let rng = StdRng::from_entropy();
        Self {
            rng: Arc::new(Mutex::new(rng)),
        }
    }
}

impl Template {
    #[must_use]
    pub fn new(rng: StdRng) -> Self {
        Self {
            rng: Arc::new(Mutex::new(rng)),
        }
    }
    /// Checks if the provided file path has a ".t" extension, marking it as a template.
    ///
    /// Returns `true` if the file has a ".t" extension, otherwise `false`.
    #[must_use]
    pub fn is_template(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .filter(|&ext| ext == TEMPLATE_EXTENSION)
            .is_some()
    }

    // Method to register filters in the Tera instance.
    fn register_filters(&self, tera_instance: &mut tera::Tera) {
        // Clone the Arc to move it into the closure.
        let rng_clone = Arc::clone(&self.rng);

        tera_instance.register_filter(
            "random_string",
            move |value: &tera::Value, _args: &HashMap<String, tera::Value>| {
                if let tera::Value::Number(length) = value {
                    if let Some(length) = length.as_u64() {
                        let rand_str: String = rng_clone.lock().map_or_else(
                            |_| {
                                let mut r = StdRng::from_entropy();
                                generate_random_string(&mut r, length)
                            },
                            |mut rng| generate_random_string(&mut *rng, length),
                        );
                        return Ok(tera::Value::String(rand_str));
                    }
                }
                // Ok(tera::Value::String(String::new()))
                Err(tera::Error::msg("arg must be a number"))
            },
        );
    }

    /// Renders a template with the provided content and settings.
    ///
    /// # Errors
    /// when could not render the template
    pub fn render(&self, template_content: &str, settings: &Settings) -> tera::Result<String> {
        tracing::trace!(
            template_content,
            settings = format!("{settings:#?}"),
            "render template"
        );

        let mut tera_instance = Tera::default();
        self.register_filters(&mut tera_instance);

        let mut context = Context::new();
        context.insert("settings", &settings);

        let rendered_output = tera_instance.render_str(template_content, &context)?;

        Ok(rendered_output)
    }

    /// Removes the ".t" extension from a template file path, if present.
    ///
    /// # Errors
    /// if the given path is not contains template extension
    pub fn strip_template_extension(&self, path: &Path) -> std::io::Result<PathBuf> {
        path.file_stem().map_or_else(
            || {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Failed to retrieve file stem",
                ))
            },
            |stem| {
                let mut path_without_extension = path.to_path_buf();
                path_without_extension.set_file_name(stem);
                if let Some(parent_dir) = path.parent() {
                    path_without_extension = parent_dir.join(stem.to_string_lossy().to_string());
                }
                Ok(path_without_extension)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_template() {
        let template = Template::default();

        let path = Path::new("example.t");
        assert!(template.is_template(path));

        let path = Path::new("example.txt");
        assert!(!template.is_template(path));

        let path = Path::new("directory/");
        assert!(!template.is_template(path));
    }

    #[test]
    fn test_render_template() {
        let template = Template::default();
        let template_content = "crate: {{ settings.package_name }}";

        let mock_settings = Settings {
            package_name: "loco-app".to_string(),
            ..Default::default()
        };

        let result = template.render(template_content, &mock_settings);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "crate: loco-app");
    }

    #[test]
    fn test_strip_template_extension() {
        let template = Template::default();

        let path = Path::new("example.t");
        let result = template.strip_template_extension(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("example"));

        let path = Path::new("example");
        let result = template.strip_template_extension(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Path::new("example"));

        let path = Path::new("");
        let result = template.strip_template_extension(path);
        assert!(result.is_err());
    }

    #[test]
    fn can_create_random_string() {
        let template = Template::default();
        let template_content = "rand: {{20 | random_string }}";

        let mock_settings = Settings {
            package_name: "loco-app".to_string(),
            ..Default::default()
        };

        let result = template.render(template_content, &mock_settings);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "rand: IhPi3oZCnaWvL2oIeA07");
        let result = template.render(template_content, &mock_settings);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "rand: mg3ZtJzh0NoAKhdDqpQ2");
    }
}
