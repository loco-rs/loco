// this is because not using with-db renders some of the structs below unused
// TODO: should be more properly aligned with extracting out the db-related gen
// code and then feature toggling it
#![allow(dead_code)]
pub use rrgen::{GenResult, RRgen};
use serde_json::{json, Value};
mod controller;
use colored::Colorize;
use std::fmt::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod column;
#[cfg(feature = "with-db")]
mod infer;
#[cfg(feature = "with-db")]
mod migration;
#[cfg(feature = "with-db")]
mod model;
#[cfg(feature = "with-db")]
mod scaffold;
pub mod template;
#[cfg(test)]
mod testutil;

#[derive(Debug)]
pub struct GenerateResults {
    rrgen: Vec<rrgen::GenResult>,
    local_templates: Vec<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("template {} not found", path.display())]
    TemplateNotFound { path: PathBuf },
    #[error(transparent)]
    RRgen(#[from] rrgen::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message(err.to_string()) //.bt()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum DeploymentKind {
    Docker {
        copy_paths: Vec<PathBuf>,
        is_client_side_rendering: bool,
    },
    Nginx {
        host: String,
        port: i32,
    },
    Lambda {
        /// Whether the app uses a database. Controls the `Migrator` import and
        /// the `create_app` generic in the generated Lambda entrypoint.
        db: bool,
        /// Runtime asset directories to bundle into the Lambda zip, alongside
        /// the always-included `config/`. Detected from the app's config so
        /// nothing is hardcoded (e.g. `assets/` when static serving is on).
        include_paths: Vec<PathBuf>,
    },
}

#[derive(Debug)]
pub enum Component {
    #[cfg(feature = "with-db")]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Whether to include timestamps (`created_at``updated_at`at columns) in the model
        with_tz: bool,

        /// Model fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    Migration {
        /// Name of the migration file
        name: String,

        /// Whether to include timestamps (`created_at`, `updated_at` columns) in the migration
        with_tz: bool,

        /// Params fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Whether to include timestamps (`created_at``updated_at`at columns) in the scaffold
        with_tz: bool,

        /// Model and params fields, eg. title:string hits:int
        fields: Vec<(String, String)>,

        /// Whether to also emit the React-SPA frontend (hooks + pages + routes
        /// injection). Set when the app has a clientside `frontend/`; when
        /// `false` only the typed backend (DTO + controller) is emitted.
        frontend: bool,

        /// Whether the generated handlers take an `auth::JWT` extractor.
        /// Scaffolds are authenticated by default (secure by default); the CLI
        /// sets this to `false` for `--no-auth`, which emits public routes.
        auth: bool,
    },
    Controller {
        /// Name of the thing to generate
        name: String,

        /// Action names
        actions: Vec<String>,

        /// Whether the generated handlers take an `auth::JWT` extractor.
        /// Unlike the scaffold, a bare controller is public by default — it has
        /// no model or DTO to protect — and the CLI sets this via `--auth`.
        auth: bool,
    },
    Task {
        /// Name of the thing to generate
        name: String,
    },
    Scheduler {},
    Worker {
        /// Name of the thing to generate
        name: String,
    },
    Mailer {
        /// Name of the thing to generate
        name: String,
    },
    Data {
        /// Name of the thing to generate
        name: String,
    },
    Deployment {
        kind: DeploymentKind,
    },
}

pub struct AppInfo {
    pub app_name: String,

    /// Root of the app being generated into.
    ///
    /// The same directory `RRgen` writes to, so post-generation checks look at
    /// the files that were just written rather than at the process's current
    /// directory. `RRgen::default()` resolves relative to the cwd, so a caller
    /// that uses it should pass `"."`.
    pub working_dir: PathBuf,
}

#[must_use]
pub fn new_generator() -> RRgen {
    RRgen::default()
}

/// Generate a component
///
/// # Errors
///
/// This function will return an error if it fails
pub fn generate(rrgen: &RRgen, component: Component, appinfo: &AppInfo) -> Result<GenerateResults> {
    /*
    (1)
    XXX: remove hooks generic from child generator, materialize it here and pass it
         means each generator accepts a [component, config, context] tuple
         this will allow us to test without an app instance
    (2) proceed to test individual generators
     */
    let get_result = match component {
        #[cfg(feature = "with-db")]
        Component::Model {
            name,
            with_tz,
            fields,
        } => model::generate(rrgen, &name, with_tz, &fields, appinfo)?,
        #[cfg(feature = "with-db")]
        Component::Scaffold {
            name,
            with_tz,
            fields,
            frontend,
            auth,
        } => scaffold::generate(rrgen, &name, with_tz, &fields, frontend, auth, appinfo)?,
        #[cfg(feature = "with-db")]
        Component::Migration {
            name,
            with_tz,
            fields,
        } => migration::generate(rrgen, &name, with_tz, &fields, appinfo)?,
        Component::Controller {
            name,
            actions,
            auth,
        } => controller::generate(rrgen, &name, &actions, auth, appinfo)?,
        Component::Task { name } => {
            let vars = json!({"name": name, "pkg_name": appinfo.app_name});
            render_template(rrgen, Path::new("task"), &vars)?
        }
        Component::Scheduler {} => {
            let vars = json!({"pkg_name": appinfo.app_name});
            render_template(rrgen, Path::new("scheduler"), &vars)?
        }
        Component::Worker { name } => {
            let vars = json!({"name": name, "pkg_name": appinfo.app_name});
            render_template(rrgen, Path::new("worker"), &vars)?
        }
        Component::Mailer { name } => {
            let vars = json!({ "name": name });
            render_template(rrgen, Path::new("mailer"), &vars)?
        }
        Component::Deployment { kind } => match kind {
            DeploymentKind::Docker {
                copy_paths,
                is_client_side_rendering,
            } => {
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "copy_paths": copy_paths,
                    "is_client_side_rendering": is_client_side_rendering,
                });
                render_template(rrgen, Path::new("deployment/docker"), &vars)?
            }
            DeploymentKind::Nginx { host, port } => {
                let host = host.replace("http://", "").replace("https://", "");
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "domain": host,
                    "port": port
                });
                render_template(rrgen, Path::new("deployment/nginx"), &vars)?
            }
            DeploymentKind::Lambda { db, include_paths } => {
                // `config/` is required by every Loco app at runtime; detected
                // asset dirs (if any) follow. This becomes the cargo-lambda
                // `include` array so the zip carries everything read from disk.
                let include = std::iter::once("config".to_string())
                    .chain(include_paths.iter().map(|p| p.display().to_string()))
                    .collect::<Vec<_>>();
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "db": db,
                    "include": include,
                });
                render_template(rrgen, Path::new("deployment/lambda"), &vars)?
            }
        },
        Component::Data { name } => {
            let vars = json!({ "name": name });
            render_template(rrgen, Path::new("data"), &vars)?
        }
    };

    #[cfg(feature = "with-db")]
    verify_migrations_registered(&appinfo.working_dir)?;

    Ok(get_result)
}

/// Path of the migrator that decides which migrations actually run.
#[cfg(feature = "with-db")]
const MIGRATOR: &str = "migration/src/lib.rs";

/// Anchor the generator injects the registration line above.
#[cfg(feature = "with-db")]
const MIGRATOR_ANCHOR: &str = "inject-above";

/// Fails if a migration exists on disk but is not registered in the migrator.
///
/// A migration is registered by two injections into [`MIGRATOR`]: a `mod`
/// declaration and a `Box::new(..)` entry in `migrations()`. rrgen 0.6 fails
/// when either injection cannot find its anchor, which covers the migration
/// being generated right now.
///
/// This covers what that cannot: a registration that went missing on an earlier
/// run or by hand. The migrator is ordinary source, and an entry deleted from it
/// leaves no trace — the migration still compiles, so nothing complains; it
/// never runs, so the table is never created; `db entities` then correctly finds
/// no table and writes an empty entity; and the failure finally surfaces as a
/// 500 on the first insert. Checking the whole directory on every generation is
/// the cheapest place to notice.
#[cfg(feature = "with-db")]
fn verify_migrations_registered(working_dir: &Path) -> Result<()> {
    let migrator_path = working_dir.join(MIGRATOR);
    // No migrator: an app generated without a database. Nothing to check.
    let Ok(migrator) = fs::read_to_string(&migrator_path) else {
        return Ok(());
    };

    let Ok(entries) = fs::read_dir(migrator_path.parent().unwrap_or(working_dir)) else {
        return Ok(());
    };

    let mut unregistered = entries
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_str()?;
            // Migration modules are `m<date>_<time>_<name>`; `lib.rs` and any
            // helper module are not.
            let is_migration = path.extension().is_some_and(|ext| ext == "rs")
                && stem.starts_with('m')
                && stem[1..].starts_with(|c: char| c.is_ascii_digit());
            is_migration.then(|| stem.to_string())
        })
        .filter(|module| {
            !migrator.contains(&format!("mod {module};"))
                || !migrator.contains(&format!("Box::new({module}::Migration)"))
        })
        .collect::<Vec<_>>();

    if unregistered.is_empty() {
        return Ok(());
    }

    unregistered.sort();
    let list = unregistered
        .iter()
        .map(|module| format!("  {module}"))
        .collect::<Vec<_>>()
        .join("\n");

    Err(Error::Message(format!(
        "these migrations exist but are not registered in {MIGRATOR}, so they will never \
         run:\n{list}\n\nEach one needs both lines:\n  mod <migration>;\n  \
         Box::new(<migration>::Migration),\n\nThe generator injects the second above the \
         `{MIGRATOR_ANCHOR}` comment in `migrations()`. If that comment is gone, add it back \
         — without it the injection silently does nothing and the table is never created."
    )))
}

fn render_template(rrgen: &RRgen, template: &Path, vars: &Value) -> Result<GenerateResults> {
    let template_files = template::collect_files_from_path(template)?;

    let mut gen_result = vec![];
    let mut local_templates = vec![];
    for template in template_files {
        let custom_template = Path::new(template::DEFAULT_LOCAL_TEMPLATE).join(template.path());

        if custom_template.exists() {
            let content = fs::read_to_string(&custom_template).inspect_err(|_err| {
                tracing::error!(custom_template = %custom_template.display(), "could not read custom template");
            })?;
            gen_result.push(rrgen.generate(&content, vars)?);
            local_templates.push(custom_template);
        } else {
            let content = template.contents_utf8().ok_or(Error::Message(format!(
                "could not get template content: {}",
                template.path().display()
            )))?;
            gen_result.push(rrgen.generate(content, vars)?);
        }
    }

    Ok(GenerateResults {
        rrgen: gen_result,
        local_templates,
    })
}

#[must_use]
pub fn collect_messages(results: &GenerateResults) -> String {
    let mut messages = String::new();

    for res in &results.rrgen {
        if let rrgen::GenResult::Generated {
            message: Some(message),
        } = res
        {
            let _ = writeln!(messages, "* {message}");
        }
    }

    if !results.local_templates.is_empty() {
        let _ = writeln!(messages);
        let _ = writeln!(
            messages,
            "{}",
            "The following templates were sourced from the local templates:".green()
        );

        for f in &results.local_templates {
            let _ = writeln!(messages, "* {}", f.display());
        }
    }
    messages
}

/// Copies template files to a specified destination directory.
///
/// This function copies files from the specified template path to the
/// destination directory. If the specified path is `/` or `.`, it copies all
/// files from the templates directory. If the path does not exist in the
/// templates, it returns an error.
///
/// # Errors
/// when could not copy the given template path
pub fn copy_template(path: &Path, to: &Path) -> Result<Vec<PathBuf>> {
    let copy_template_path = if path == Path::new("/") || path == Path::new(".") {
        None
    } else if !template::exists(path) {
        return Err(Error::TemplateNotFound {
            path: path.to_path_buf(),
        });
    } else {
        Some(path)
    };

    let copy_files = if let Some(path) = copy_template_path {
        template::collect_files_from_path(path)?
    } else {
        template::collect_files()
    };

    let mut copied_files = vec![];
    for f in copy_files {
        let copy_to = to.join(f.path());
        if copy_to.exists() {
            tracing::debug!(
                template_file = %copy_to.display(),
                "skipping copy template file. already exists"
            );
            continue;
        }
        match copy_to.parent() {
            Some(parent) => {
                fs::create_dir_all(parent)?;
            }
            None => {
                return Err(Error::Message(format!(
                    "could not get parent folder of {}",
                    copy_to.display()
                )));
            }
        }

        fs::write(&copy_to, f.contents())?;
        tracing::trace!(
            template = %copy_to.display(),
            "copy template successfully"
        );
        copied_files.push(copy_to);
    }
    Ok(copied_files)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_template_not_found() {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("create temp file");
        let path = Path::new("nonexistent-template");

        let result = copy_template(path, tree_fs.root.as_path());
        assert!(result.is_err());
        if let Err(Error::TemplateNotFound { path: p }) = result {
            assert_eq!(p, path.to_path_buf());
        } else {
            panic!("Expected TemplateNotFound error");
        }
    }

    #[test]
    fn test_copy_template_valid_folder_template() {
        let temp_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .create()
            .expect("Failed to create temporary file system");

        let template_dir = template::tests::find_first_dir();

        let copy_result = copy_template(template_dir.path(), temp_fs.root.as_path());
        assert!(
            copy_result.is_ok(),
            "Failed to copy template from directory {:?}",
            template_dir.path()
        );

        let template_files = template::collect_files_from_path(template_dir.path())
            .expect("Failed to collect files from the template directory");

        assert!(
            !template_files.is_empty(),
            "No files found in the template directory"
        );

        for template_file in template_files {
            let copy_file_path = temp_fs.root.join(template_file.path());

            assert!(
                copy_file_path.exists(),
                "Copy file does not exist: {copy_file_path:?}"
            );

            let copy_content =
                fs::read_to_string(&copy_file_path).expect("Failed to read coped file content");

            assert_eq!(
                template_file
                    .contents_utf8()
                    .expect("Failed to get template file content"),
                copy_content,
                "Content mismatch in file: {copy_file_path:?}"
            );
        }
    }

    #[test]
    fn can_collect_messages() {
        let gen_result = GenerateResults {
            rrgen: vec![
                GenResult::Skipped,
                GenResult::Generated {
                    message: Some("test".to_string()),
                },
                GenResult::Generated {
                    message: Some("test2".to_string()),
                },
                GenResult::Generated { message: None },
            ],
            local_templates: vec![
                PathBuf::from("template").join("scheduler.t"),
                PathBuf::from("template").join("task.t"),
            ],
        };

        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();

        assert_eq!(
            re.replace_all(&collect_messages(&gen_result), ""),
            r"* test
* test2

The following templates were sourced from the local templates:
* template/scheduler.t
* template/task.t
"
        );
    }
}
