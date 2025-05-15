//! This module defines the `Generator` struct, which is responsible for
//! executing scripted commands

use std::path::Path;
pub mod executer;
pub mod template;
use std::sync::Arc;

use include_dir::{include_dir, Dir};
use rhai::{
    export_module, exported_module,
    plugin::{
        Dynamic, FnNamespace, FuncRegistration, Module, NativeCallContext, PluginFunc, RhaiResult,
        TypeId,
    },
    Engine, Scope,
};

use crate::wizard::AssetsOption;
use crate::{settings, OS};

static APP_TEMPLATE: Dir<'_> = include_dir!("base_template");

/// Extracts a default template to a temporary directory for use by the
/// application.
///
/// # Errors
/// when could not extract the the base template
pub fn extract_default_template() -> std::io::Result<tree_fs::Tree> {
    let generator_tmp_folder = tree_fs::TreeBuilder::default().create()?;

    APP_TEMPLATE.extract(&generator_tmp_folder.root)?;
    Ok(generator_tmp_folder)
}

/// The `Generator` struct provides functionality to execute scripted
/// operations, such as copying files and templates, based on the current
/// settings.
#[derive(Clone)]
pub struct Generator {
    pub executer: Arc<dyn executer::Executer>,
    pub settings: settings::Settings,
}
impl Generator {
    /// Creates a new [`Generator`] with a given executor and settings.
    pub fn new(executer: Arc<dyn executer::Executer>, settings: settings::Settings) -> Self {
        Self { executer, settings }
    }

    /// Runs the default script.
    ///
    /// # Errors
    ///
    /// Returns an error if the script execution fails.
    pub fn run(&self) -> crate::Result<()> {
        self.run_from_script(include_str!("../../setup.rhai"))
    }

    /// Runs a custom script provided as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the script execution fails.
    pub fn run_from_script(&self, script: &str) -> crate::Result<()> {
        let mut engine = Engine::new();

        tracing::debug!(
            settings = format!("{:?}", self.settings),
            script,
            "prepare installation script"
        );
        engine
            .build_type::<settings::Settings>()
            .register_type_with_name::<Option<settings::Initializers>>("Option<Initializers>")
            .register_type_with_name::<Option<settings::Asset>>("Option<settings::Asset>")
            .register_static_module(
                "rhai_settings_extensions",
                exported_module!(rhai_settings_extensions).into(),
            )
            .register_fn("copy_file", Self::copy_file)
            .register_fn("create_file", Self::create_file)
            .register_fn("copy_files", Self::copy_files)
            .register_fn("copy_dir", Self::copy_dir)
            .register_fn("copy_dirs", Self::copy_dirs)
            .register_fn("copy_template", Self::copy_template)
            .register_fn("copy_template_dir", Self::copy_template_dir);

        let settings_dynamic = rhai::Dynamic::from(self.settings.clone());

        let mut scope = Scope::new();
        scope.set_value("settings", settings_dynamic);
        scope.push("gen", self.clone());
        // TODO:: move it as part of the settings?
        scope.push("db", self.settings.db.is_some());
        scope.push("background", self.settings.background.is_some());
        scope.push("initializers", self.settings.initializers.is_some());
        scope.push("asset", self.settings.asset.is_some());
        scope.push("windows", self.settings.os == OS::Windows);

        engine.run_with_scope(&mut scope, script)?;
        Ok(())
    }

    /// Copies a single file from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file copy operation fails.
    pub fn copy_file(&mut self, path: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_file", path);
        let _guard = span.enter();

        self.executer.copy_file(Path::new(path)).map_err(|err| {
            Box::new(rhai::EvalAltResult::ErrorSystem(
                "copy_file".to_string(),
                err.into(),
            ))
        })?;
        Ok(())
    }

    /// Creates a single file in the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file copy operation fails.
    pub fn create_file(
        &mut self,
        path: &str,
        content: &str,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("create_file", path);
        let _guard = span.enter();

        self.executer
            .create_file(Path::new(path), content.to_string())
            .map_err(|err| {
                Box::new(rhai::EvalAltResult::ErrorSystem(
                    "create_file".to_string(),
                    err.into(),
                ))
            })?;
        Ok(())
    }

    /// Copies list of files from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file copy operation fails.
    pub fn copy_files(&mut self, paths: rhai::Array) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_files");
        let _guard = span.enter();
        for path in paths {
            self.executer
                .copy_file(Path::new(&path.to_string()))
                .map_err(|err| {
                    Box::new(rhai::EvalAltResult::ErrorSystem(
                        "copy_files".to_string(),
                        err.into(),
                    ))
                })?;
        }

        Ok(())
    }

    /// Copies an entire directory from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory copy operation fails.
    pub fn copy_dir(&mut self, path: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_dir", path);
        let _guard = span.enter();
        self.executer.copy_dir(Path::new(path)).map_err(|err| {
            Box::new(rhai::EvalAltResult::ErrorSystem(
                "copy_dir".to_string(),
                err.into(),
            ))
        })
    }

    /// Copies list of directories from the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory copy operation fails.
    pub fn copy_dirs(&mut self, paths: rhai::Array) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_dirs");
        let _guard = span.enter();
        for path in paths {
            self.executer
                .copy_dir(Path::new(&path.to_string()))
                .map_err(|err| {
                    Box::new(rhai::EvalAltResult::ErrorSystem(
                        "copy_dirs".to_string(),
                        err.into(),
                    ))
                })?;
        }
        Ok(())
    }

    /// Copies a template file from the specified path, applying settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the template copy operation fails.
    pub fn copy_template(&mut self, path: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_template", path);
        let _guard = span.enter();
        self.executer
            .copy_template(Path::new(path), &self.settings)
            .map_err(|err| {
                Box::new(rhai::EvalAltResult::ErrorSystem(
                    "copy_template".to_string(),
                    err.into(),
                ))
            })
    }

    /// Copies an entire template directory from the specified path, applying
    /// settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the template directory copy operation fails.
    pub fn copy_template_dir(&mut self, path: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        let span = tracing::info_span!("copy_template_dir", path);
        let _guard = span.enter();
        self.executer
            .copy_template_dir(Path::new(path), &self.settings)
            .map_err(|err| {
                Box::new(rhai::EvalAltResult::ErrorSystem(
                    "copy_template_dir".to_string(),
                    err.into(),
                ))
            })
    }
}

/// This module provides extensions to the [`rhai`] scripting language,
/// enabling ergonomic access to specific.
/// These extensions allow scripts to interact with internal settings
/// in a controlled and expressive way.
#[export_module]
mod rhai_settings_extensions {

    /// Retrieves the value of the `view_engine` field from the [`settings::Initializers`] struct.
    #[rhai_fn(global, get = "view_engine", pure)]
    pub fn view_engine(initializers: &mut Option<settings::Initializers>) -> bool {
        initializers.as_ref().is_some_and(|i| i.view_engine)
    }

    /// Checks if the rendering method is set to client-side rendering.
    #[rhai_fn(global, get = "is_client_side", pure)]
    pub fn is_client_side(rendering_method: &mut Option<settings::Asset>) -> bool {
        rendering_method
            .as_ref()
            .is_some_and(|r| matches!(r.kind, AssetsOption::Clientside))
    }

    /// Checks if the rendering method is set to server-side rendering.
    #[rhai_fn(global, get = "is_server_side", pure)]
    pub fn is_server_side(rendering_method: &mut Option<settings::Asset>) -> bool {
        rendering_method
            .as_ref()
            .is_some_and(|r| matches!(r.kind, AssetsOption::Serverside))
    }
}

#[cfg(test)]
mod tests {
    use executer::MockExecuter;
    use mockall::predicate::*;

    use super::*;

    #[test]
    pub fn can_copy_file() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_file()
            .with(eq(Path::new("test.rs")))
            .times(1)
            .returning(|p| Ok(p.to_path_buf()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res = g.run_from_script(r#"gen.copy_file("test.rs");"#);

        assert!(script_res.is_ok());
    }

    #[test]
    pub fn can_copy_files() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_file()
            .with(eq(Path::new(".gitignore")))
            .times(1)
            .returning(|p| Ok(p.to_path_buf()));

        executor
            .expect_copy_file()
            .with(eq(Path::new(".rustfmt.toml")))
            .times(1)
            .returning(|p| Ok(p.to_path_buf()));

        executor
            .expect_copy_file()
            .with(eq(Path::new("README.md")))
            .times(1)
            .returning(|p| Ok(p.to_path_buf()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res =
            g.run_from_script(r#"gen.copy_files([".gitignore", ".rustfmt.toml", "README.md"]);"#);

        assert!(script_res.is_ok());
    }

    #[test]
    pub fn can_copy_dir() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_dir()
            .with(eq(Path::new("test")))
            .times(1)
            .returning(|_| Ok(()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res = g.run_from_script(r#"gen.copy_dir("test");"#);

        assert!(script_res.is_ok());
    }

    #[test]
    pub fn can_copy_dirs() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_dir()
            .with(eq(Path::new("src")))
            .times(1)
            .returning(|_| Ok(()));

        executor
            .expect_copy_dir()
            .with(eq(Path::new("example")))
            .times(1)
            .returning(|_| Ok(()));

        executor
            .expect_copy_dir()
            .with(eq(Path::new(".github")))
            .times(1)
            .returning(|_| Ok(()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res = g.run_from_script(r#"gen.copy_dirs(["src", "example", ".github"]);"#);

        assert!(script_res.is_ok());
    }

    #[test]
    pub fn can_copy_template() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_template()
            .with(eq(Path::new("src/lib.rs.t")), always())
            .times(1)
            .returning(|_, _| Ok(()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res = g.run_from_script(r#"gen.copy_template("src/lib.rs.t");"#);

        assert!(script_res.is_ok());
    }

    #[test]
    pub fn can_copy_template_dir() {
        let mut executor = MockExecuter::new();

        executor
            .expect_copy_template_dir()
            .with(eq(Path::new("src/examples")), always())
            .times(1)
            .returning(|_, _| Ok(()));

        let g = Generator::new(Arc::new(executor), settings::Settings::default());
        let script_res = g.run_from_script(r#"gen.copy_template_dir("src/examples");"#);

        assert!(script_res.is_ok());
    }
}
