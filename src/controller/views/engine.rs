use std::path::{Path, PathBuf};

use super::tera_builtins;
use crate::{controller::views::ViewRenderer, Error, Result};
use serde::Serialize;
use std::hash::{DefaultHasher, Hash, Hasher};

pub static DEFAULT_ASSET_FOLDER: &str = "assets";

#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub struct HotReloadingTeraEngine {
    pub engine: tera::Tera,
    pub view_path: PathBuf,
    pub view_path_hash: u64,
}

#[derive(Clone)]
pub struct TeraView {
    #[cfg(debug_assertions)]
    pub tera: std::sync::Arc<std::sync::Mutex<HotReloadingTeraEngine>>,

    #[cfg(not(debug_assertions))]
    pub tera: tera::Tera,

    pub tera_post_process:
        Option<std::sync::Arc<dyn Fn(&mut tera::Tera) -> Result<()> + Send + Sync>>,

    pub default_context: tera::Context,
}

impl std::fmt::Debug for TeraView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeraView")
            .field("tera", &self.tera)
            .field(
                "tera_post_process",
                if self.tera_post_process.is_some() {
                    &Some("Fn")
                } else {
                    &None::<&'static str>
                },
            )
            .field("default_context", &self.default_context)
            .finish()
    }
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

    /// Attach the Tera view engine with a post-processing function for subsequent instantiation.
    ///
    /// The post-processing function is also run during the call to this method.
    ///
    /// # Errors
    ///
    /// This function will return an error if the post-processing function fails
    pub fn post_process(
        mut self,
        post_process: impl Fn(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        {
            #[cfg(debug_assertions)]
            let engine = &mut self.tera.lock().unwrap().engine;

            #[cfg(not(debug_assertions))]
            let engine = &mut self.tera;

            post_process(engine)?;
        }

        self.tera_post_process = Some(std::sync::Arc::new(post_process));
        Ok(self)
    }

    /// Create a new Tera instance from a directory path
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    fn create_tera_instance<P: AsRef<Path>>(path: P) -> Result<tera::Tera> {
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::string("invalid glob"))?;

        let mut tera = tera::Tera::new(path)?;

        tera_builtins::filters::register_filters(&mut tera);

        Ok(tera)
    }

    /// Create a unique hash for the view directory based on the following
    /// metadata of the files in the directory:
    ///
    /// 1) The file name
    /// 2) The file size
    /// 3) The last modified time
    ///
    /// If this hash changes, it indicates that at least one of the files
    /// in the directory has changed,
    ///
    /// # Note
    ///
    /// This glob-walking code is taken directly from Tera because
    /// we want to ensure that we handle glob patterns consistently
    fn hash_view_dir<P: AsRef<Path>>(path: &P) -> Result<u64> {
        let glob = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::string("invalid glob"))?;

        let Some(n) = glob.find('*') else {
            return Err(Error::string("invalid glob"));
        };

        // Copied from Tera code
        let (parent_dir, glob_end) = glob.split_at(n);
        let parent_dir = std::fs::canonicalize(parent_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from(parent_dir));

        let glob = parent_dir
            .join(glob_end)
            .into_os_string()
            .into_string()
            .unwrap();

        let glob_walker = globwalk::glob_builder(&glob)
            .follow_links(true)
            .build()
            .map_err(|_| Error::string("error walking glob"))?;

        let mut hasher = DefaultHasher::new();

        for entry in glob_walker.filter_map(std::result::Result::ok) {
            let filename = entry.path().to_string_lossy();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            filename.hash(&mut hasher);
            metadata.len().hash(&mut hasher);
            let duration_since_epoch = modified
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            duration_since_epoch.hash(&mut hasher);
        }
        Ok(hasher.finish())
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

        let path: PathBuf = path.as_ref().join("**").join("*.html").into();

        // Hash the view path files
        let hash = Self::hash_view_dir(&path)?;

        // Create instance
        let tera = Self::create_tera_instance(&path)?;

        Ok(Self {
            tera_post_process: None,

            #[cfg(debug_assertions)]
            tera: std::sync::Arc::new(std::sync::Mutex::new(HotReloadingTeraEngine {
                engine: tera,
                view_path: path,
                view_path_hash: hash,
            })),
            #[cfg(not(debug_assertions))]
            tera: tera,

            default_context: tera::Context::default(),
        })
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let context = tera::Context::from_serialize(data)?;

        #[cfg(debug_assertions)]
        {
            let mut tera = self.tera.lock().unwrap();

            // Hash the view path files
            let hash = Self::hash_view_dir(&tera.view_path)?;

            // Only create a new Tera instance if the hash has changed
            if tera.view_path_hash != hash {
                tracing::warn!("Tera rendering in non-optimized debug mode");
                tracing::debug!(key = key, "Hot-reloading Tera view engine");

                tera.view_path_hash = hash;

                let mut new_engine = Self::create_tera_instance(&tera.view_path)?;

                if let Some(post_process) = self.tera_post_process.as_deref() {
                    post_process(&mut new_engine)?;
                }

                tera.engine = new_engine;
            }

            Ok(tera.engine.render(key, &context)?)
        }

        #[cfg(not(debug_assertions))]
        Ok(self.tera.render(key, &context)?)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use tree_fs;

    use super::*;
    #[test]
    fn can_render_view() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/test.html", "generate test.html file: {{foo}}")
            .add_file("template/test2.html", "generate test2.html file: {{bar}}")
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root).unwrap();

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

    #[cfg(debug_assertions)]
    #[test]
    fn template_inheritance_hot_reload() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file(
                "template/base.html",
                r"<!DOCTYPE html>
            <html>
            <head>
                <title>{% block title %}Default Title{% endblock %}</title>
            </head>
            <body>
                <header>Base Header v1: {{ 1 | hello }}</header>
                {% block content %}
                Default content
                {% endblock %}
                <footer>Base Footer</footer>
            </body>
            </html>",
            )
            .add_file(
                "template/child.html",
                r"{% extends 'template/base.html' %}
            {% block title %}Child Page{% endblock %}
            {% block content %}
            <div>Child content</div>
            {% endblock %}",
            )
            .create()
            .unwrap();

        let tree_dir = tree_fs.root.clone();
        let v = TeraView::from_custom_dir(&tree_fs.root)
            .unwrap()
            .post_process(|tera| {
                tera.register_filter("hello", |value: &Value, _: &HashMap<String, Value>| {
                    Ok(format!("Hello World v{value}").into())
                });
                Ok(())
            })
            .unwrap();

        // Initial render should have the original header from base template
        let initial_render = v.render("template/child.html", json!({})).unwrap();
        assert!(initial_render.contains("Base Header v1: Hello World v1"));
        assert!(initial_render.contains("Child Page"));
        assert!(initial_render.contains("Child content"));

        // Now modify the base template to change the header
        let updated_base = r"<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}Default Title{% endblock %}</title>
</head>
<body>
    <header>Base Header v2: {{ 2 | hello }}</header>
    {% block content %}
    Default content
    {% endblock %}
    <footer>Base Footer</footer>
</body>
</html>";

        // Update the base template file
        std::fs::write(
            Path::new(&tree_dir).join("template").join("base.html"),
            updated_base,
        )
        .unwrap();

        // Render again - should have the updated header due to hot reload
        let updated_render = v.render("template/child.html", json!({})).unwrap();
        assert!(updated_render.contains("Base Header v2: Hello World v2")); // Should have changed
        assert!(updated_render.contains("Child Page")); // Should be the same
        assert!(updated_render.contains("Child content")); // Should be the same
    }
}
