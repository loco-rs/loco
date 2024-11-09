use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use tower_livereload::LiveReloadLayer;

use notify_debouncer_full::{
    new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer, FileIdMap,
};
use serde::Serialize;

use crate::{controller::views::ViewRenderer, Error, Result};

const VIEWS_DIR: &str = "assets/views";

#[derive(Clone, Debug)]
pub struct ReloadableTera {
    #[cfg(debug_assertions)]
    _debouncer: Arc<Debouncer<RecommendedWatcher, FileIdMap>>,

    #[cfg(debug_assertions)]
    pub tera: Arc<RwLock<tera::Tera>>,

    // #[cfg(debug_assertions)]
    // pub reload_layer: LiveReloadLayer,

    #[cfg(not(debug_assertions))]
    pub tera: tera::Tera,
}

impl ReloadableTera {
    pub fn new(root_path: &str, path_str: &str) -> Result<Self> {
        let tera = tera::Tera::new(path_str)?;

        #[cfg(debug_assertions)]
        let (debouncer, tera) = {
            let tera = Arc::new(RwLock::new(tera));
            let debouncer =
                Self::create_reload_debouncer(Duration::from_millis(500), tera.clone(), root_path);

            (Arc::new(debouncer), tera)
        };

        Ok(Self {
            #[cfg(debug_assertions)]
            _debouncer: debouncer,
            tera,
        })
    }

    fn create_reload_debouncer(
        delay: Duration,
        tera: Arc<RwLock<tera::Tera>>,
        root_path: &str,
    ) -> Debouncer<RecommendedWatcher, FileIdMap> {
        let mut debouncer =
            new_debouncer(
                delay,
                None,
                move |result: DebounceEventResult| match result {
                    Ok(events) => events.iter().for_each(|event| match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            let mut write_lock =
                                tera.write().expect("Failed to acquire tera write lock");

                            write_lock.full_reload().expect("Failed to full reload");
                        }
                        _ => {}
                    }),
                    Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
                },
            )
            .unwrap();

        debouncer
            .watch(root_path, RecursiveMode::Recursive)
            .unwrap();

        debouncer
    }

    #[cfg(debug_assertions)]
    pub fn get(&self) -> std::sync::RwLockReadGuard<tera::Tera> {
        self.tera.read().unwrap()
    }

    #[cfg(debug_assertions)]
    pub fn get_mut(&mut self) -> std::sync::RwLockWriteGuard<tera::Tera> {
        self.tera.write().unwrap()
    }

    #[cfg(not(debug_assertions))]
    pub fn get(&self) -> &tera::Tera {
        &self.tera
    }

    #[cfg(not(debug_assertions))]
    pub fn get_mut(&mut self) -> &mut tera::Tera {
        &mut self.tera
    }
}

#[derive(Clone, Debug)]
pub struct TeraView {
    pub tera: ReloadableTera,
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

        let tera = ReloadableTera::new(
            path.as_ref()
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
            path.as_ref()
                .join("**")
                .join("*.html")
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
        )?;

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

        let tera = self.tera.get();

        Ok(tera.render(key, &context)?)
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
