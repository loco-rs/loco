use std::path::{Path, PathBuf};

use crate::{controller::views::ViewRenderer, Error, Result};
use serde::Serialize;

#[cfg(debug_assertions)]
use notify::{
    event::{EventKind, ModifyKind},
    Event, RecursiveMode, Watcher,
};

pub static DEFAULT_ASSET_FOLDER: &str = "assets";

/// A boxed post-processing function applied to a [`tera::Tera`] instance.
#[cfg(debug_assertions)]
pub type PostProcessFn = Box<dyn Fn(&mut tera::Tera) -> Result<()> + Send + Sync>;

#[cfg(debug_assertions)]
pub struct HotReloadingTeraEngine {
    pub engine: tera::Tera,
    pub view_path: PathBuf,
    pub file_watcher: Box<dyn notify::Watcher + Send + Sync>,
    pub dirty: bool,
    pub post_process: PostProcessFn,
}

#[derive(Clone)]
pub struct TeraView(
    #[cfg(debug_assertions)] std::sync::Arc<std::sync::Mutex<HotReloadingTeraEngine>>,
    #[cfg(not(debug_assertions))] std::sync::Arc<tera::Tera>,
);

impl TeraView {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        Self::from_custom_dir(&PathBuf::from(DEFAULT_ASSET_FOLDER).join("views"), |_| {
            Ok(())
        })
    }

    /// Create a Tera view engine with a post-processing function for subsequent instantiation.
    ///
    /// The post-processing function is also run during the call to this method.
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails or if the post-processing function fails
    pub fn build_with_post_process(
        post_process: impl Fn(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        Self::from_custom_dir(
            &PathBuf::from(DEFAULT_ASSET_FOLDER).join("views"),
            post_process,
        )
    }

    /// Create a new Tera instance from a directory path.
    ///
    /// `post_process` runs BEFORE templates are loaded: Tera 2 resolves filter
    /// and function references when a template is added, so anything the
    /// templates call — an i18n `t()`, a custom filter — must already be
    /// registered or the load fails with `Unknown filter`. (Tera 1 resolved
    /// these lazily at render time, so the order did not matter there.)
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    fn create_tera_instance<P: AsRef<Path>>(
        path: P,
        post_process: &(impl Fn(&mut tera::Tera) -> Result<()> + ?Sized),
    ) -> Result<tera::Tera> {
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::string("invalid glob"))?;

        let mut tera = crate::tera::instance();
        post_process(&mut tera)?;

        // Read every match first, then register the whole set in ONE call.
        // Tera 2 also resolves inheritance as templates are added and rejects a
        // child whose parent it has not seen yet, so adding them one at a time
        // would break any `{% extends %}` where the child sorts before its
        // parent.
        //
        // `load_from_glob` pairs each match with the name Tera 1's glob
        // constructor gave it (path relative to the glob base, e.g.
        // `home/hello.html`), so existing `render(...)` calls are unaffected.
        let mut templates = Vec::new();
        for (file, name) in tera::load_from_glob(path)? {
            templates.push((name, std::fs::read_to_string(&file)?));
        }
        tera.add_raw_templates(templates)?;

        Ok(tera)
    }

    /// Create a Tera view engine from a custom directory
    ///
    /// The post-processing function is also run during the call to this method.
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails or if the post-processing function fails
    pub fn from_custom_dir<P: AsRef<Path>>(
        path: &P,
        post_process: impl Fn(&mut tera::Tera) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::string(&format!(
                "missing views directory: `{}`",
                path.as_ref().display()
            )));
        }
        let view_dir = path.as_ref();
        let view_path: PathBuf = view_dir.join("**").join("*.html");

        // Create instance. `post_process` runs inside, before templates load —
        // see `create_tera_instance`.
        let tera = Self::create_tera_instance(&view_path, &post_process)?;

        // Enable hot-reloading in debug build
        #[cfg(debug_assertions)]
        let tera = {
            let tera = std::sync::Arc::new(std::sync::Mutex::new(HotReloadingTeraEngine {
                engine: tera,
                view_path,
                file_watcher: Box::new(notify::NullWatcher),
                dirty: false,
                post_process: Box::new(post_process),
            }));

            let tera2 = tera.clone();

            // Create file watcher
            let mut watcher = notify::recommended_watcher(move |event| {
                use tracing::info;

                let Ok(Event { kind, paths, .. }) = event else {
                    return;
                };

                // Only handle sub-directories and .html files
                if !paths
                    .iter()
                    .all(|p| p.is_dir() || p.extension().is_some_and(|ext| ext == "html"))
                {
                    return;
                }

                // Set dirty flag if file/directory modified
                match kind {
                    // Simple access, no changes
                    EventKind::Access(_) => return,
                    // Metadata changes, no content change
                    EventKind::Modify(ModifyKind::Metadata(_)) => return,
                    // Content modified
                    EventKind::Modify(ModifyKind::Data(change)) => {
                        info!(?paths, ?change, "View file modified")
                    }
                    // File renamed
                    EventKind::Modify(ModifyKind::Name(change)) => {
                        info!(?paths, ?change, "View file renamed")
                    }
                    // Other modifications
                    EventKind::Modify(change) => {
                        info!(?paths, ?change, "View file modified")
                    }
                    // File created.
                    EventKind::Create(_) => info!(?paths, "View file created"),
                    // File removed.
                    EventKind::Remove(_) => info!(?paths, "View file removed"),
                    // All other changes.
                    change => info!(?paths, ?change, "View file changed"),
                }

                tera2
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .dirty = true;
            })
            .map_err(|_| Error::string("error creating file watcher"))?;

            watcher
                .watch(view_dir, RecursiveMode::Recursive)
                .map_err(|_| Error::string("error watching for file changes in view directory"))?;

            tera.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .file_watcher = Box::new(watcher);
            tera
        };

        #[cfg(not(debug_assertions))]
        let tera = std::sync::Arc::new(tera);

        Ok(Self(tera))
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let context = tera::Context::from_serialize(&data)?;

        #[cfg(debug_assertions)]
        {
            let mut tera = self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            // Only create a new Tera instance if the view path files have changed
            if tera.dirty {
                tracing::warn!(key, "Hot-reloading Tera view engine");

                tera.dirty = false;

                let new_engine =
                    Self::create_tera_instance(&tera.view_path, tera.post_process.as_ref())?;

                tera.engine = new_engine;
            }

            Ok(tera.engine.render(key, &context)?)
        }

        #[cfg(not(debug_assertions))]
        Ok(self.0.render(key, &context)?)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tree_fs;

    use super::*;
    #[test]
    fn can_render_view() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/test.html", "generate test.html file: {{foo}}")
            .add_file("template/test2.html", "generate test2.html file: {{bar}}")
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |_| Ok(())).unwrap();

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

    /// A custom filter registered through `post_process` must be usable BY the
    /// templates. Tera 2 resolves filter references when a template is added,
    /// so if registration ran after loading (as it did before this was fixed),
    /// every app with a custom filter — or the standard i18n `t()` function —
    /// failed at startup with `Unknown filter`.
    #[test]
    fn post_process_registrations_are_visible_to_templates() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/uses_filter.html", "{{ 'x' | shout }}")
            .add_file("template/uses_fn.html", "{{ greet(who='world') }}")
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |tera| {
            tera.register_filter(
                "shout",
                |value: &tera::Value, _: tera::Kwargs, _: &tera::State| {
                    tera::TeraResult::Ok(tera::Value::from(value.to_string().to_uppercase()))
                },
            );
            tera.register_function("greet", |kwargs: tera::Kwargs, _: &tera::State| {
                let who: String = kwargs.must_get("who")?;
                tera::TeraResult::Ok(tera::Value::from(format!("hello {who}")))
            });
            Ok(())
        })
        .unwrap();

        assert_eq!(
            v.render("template/uses_filter.html", json!({})).unwrap(),
            "X"
        );
        assert_eq!(
            v.render("template/uses_fn.html", json!({})).unwrap(),
            "hello world"
        );
    }

    /// `get_env` was a Tera 1 built-in available in every template, including
    /// views. Tera 2 dropped it, so Loco registers its own — this guards the
    /// parity for the view engine specifically.
    #[test]
    fn get_env_is_available_in_views() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file(
                "template/env.html",
                r#"{{ get_env(name="LOCO_VIEW_NOPE", default="fallback") }}"#,
            )
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |_| Ok(())).unwrap();
        assert_eq!(
            v.render("template/env.html", json!({})).unwrap(),
            "fallback"
        );
    }

    /// Inheritance across files must work regardless of the order the glob
    /// yields them: Tera 2 rejects a child added before its parent, so the
    /// whole set has to be registered in one call.
    #[test]
    fn inheritance_works_when_child_sorts_before_parent() {
        // "a_child" sorts before "z_base", so a naive per-file loop would add
        // the child first and fail.
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file(
                "template/z_base.html",
                "<body>{% block content %}base{% endblock %}</body>",
            )
            .add_file(
                "template/a_child.html",
                "{% extends 'template/z_base.html' %}{% block content %}child{% endblock %}",
            )
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |_| Ok(())).unwrap();
        assert_eq!(
            v.render("template/a_child.html", json!({})).unwrap(),
            "<body>child</body>"
        );
    }

    /// The built-in number filters must be reachable from a real template, not
    /// just callable as functions — they are registered through the same path
    /// that Tera 2 validates at add time.
    #[test]
    fn builtin_filters_are_available_in_views() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/n.html", "{{ n | number_with_delimiter }}")
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |_| Ok(())).unwrap();
        assert_eq!(
            v.render("template/n.html", json!({"n": 1_234_567}))
                .unwrap(),
            "1,234,567"
        );
    }

    /// HTML autoescaping must stay on for `.html`/`.htm`/`.xml` views. Tera
    /// applies it by template-name suffix, and the Tera 2 migration changed how
    /// views are registered (glob constructor -> `add_raw_template`), so this
    /// guards against silently rendering user data unescaped.
    #[test]
    fn html_views_autoescape_interpolated_values() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/xss.html", "<p>{{ evil }}</p>")
            .create()
            .unwrap();

        let v = TeraView::from_custom_dir(&tree_fs.root, |_| Ok(())).unwrap();
        let out = v
            .render(
                "template/xss.html",
                json!({"evil": "<script>alert('x')</script>"}),
            )
            .unwrap();

        assert!(
            !out.contains("<script>"),
            "view output was not HTML-escaped: {out}"
        );
        assert!(out.contains("&lt;script&gt;"), "unexpected escaping: {out}");
    }

    /// The counterpart: non-markup templates must NOT be escaped, or plain-text
    /// output (e.g. `.txt` mail bodies) would grow entities.
    #[test]
    fn non_html_views_are_not_escaped() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .add_file("template/plain.txt", "{{ raw }}")
            .create()
            .unwrap();

        // `.txt` is outside the glob the view engine loads, so register it the
        // same way the engine does and assert on suffix behaviour directly.
        let mut tera = crate::tera::instance();
        tera.add_raw_template("plain.txt", "{{ raw }}").unwrap();
        let ctx = tera::Context::from_serialize(&json!({"raw": "a < b & c"})).unwrap();
        assert_eq!(tera.render("plain.txt", &ctx).unwrap(), "a < b & c");
        drop(tree_fs);
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
        let v = TeraView::from_custom_dir(&tree_fs.root, |tera| {
            tera.register_filter(
                "hello",
                |value: &tera::Value, _: tera::Kwargs, _: &tera::State| {
                    tera::TeraResult::Ok(tera::Value::from(format!("Hello World v{value}")))
                },
            );
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

        // Wait for file watcher to detect the change
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Render again - should have the updated header due to hot reload
        let updated_render = v.render("template/child.html", json!({})).unwrap();
        assert!(updated_render.contains("Base Header v2: Hello World v2")); // Should have changed
        assert!(updated_render.contains("Child Page")); // Should be the same
        assert!(updated_render.contains("Child content")); // Should be the same
    }
}
