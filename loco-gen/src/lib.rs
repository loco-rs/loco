// this is because not using with-db renders some of the structs below unused
// TODO: should be more properly aligned with extracting out the db-related gen
// code and then feature toggling it
#![allow(dead_code)]
pub use rrgen::{GenResult, RRgen};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
mod controller;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::OnceLock,
};

use colored::Colorize;

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
const DEPLOYMENT_SHUTTLE_RUNTIME_VERSION: &str = "0.46.0";

const DEPLOYMENT_OPTIONS: &[(&str, DeploymentKind)] = &[
    ("Docker", DeploymentKind::Docker),
    ("Shuttle", DeploymentKind::Shuttle),
    ("Nginx", DeploymentKind::Nginx),
];

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

#[derive(Serialize, Deserialize, Debug)]
struct FieldType {
    name: String,
    rust: Option<String>,
    schema: Option<String>,
    col_type: Option<String>,
    #[serde(default)]
    arity: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mappings {
    field_types: Vec<FieldType>,
}
impl Mappings {
    pub fn rust_field(&self, field: &str) -> Option<&String> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .and_then(|f| f.rust.as_ref())
    }
    pub fn schema_field(&self, field: &str) -> Option<&String> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .and_then(|f| f.schema.as_ref())
    }
    pub fn col_type_field(&self, field: &str) -> Option<&String> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .and_then(|f| f.col_type.as_ref())
    }
    pub fn col_type_arity(&self, field: &str) -> Option<usize> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.arity)
    }
    pub fn schema_fields(&self) -> Vec<&String> {
        self.field_types
            .iter()
            .filter(|f| f.schema.is_some())
            .map(|f| &f.name)
            .collect::<Vec<_>>()
    }
    pub fn rust_fields(&self) -> Vec<&String> {
        self.field_types
            .iter()
            .filter(|f| f.rust.is_some())
            .map(|f| &f.name)
            .collect::<Vec<_>>()
    }
}

static MAPPINGS: OnceLock<Mappings> = OnceLock::new();

fn get_mappings() -> &'static Mappings {
    MAPPINGS.get_or_init(|| {
        let json_data = include_str!("./mappings.json");
        serde_json::from_str(json_data).expect("JSON was not well-formatted")
    })
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ScaffoldKind {
    Api,
    Html,
    Htmx,
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum DeploymentKind {
    Docker,
    Shuttle,
    Nginx,
}
impl FromStr for DeploymentKind {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(Self::Docker),
            "shuttle" => Ok(Self::Shuttle),
            "nginx" => Ok(Self::Nginx),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Component {
    #[cfg(feature = "with-db")]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Is it a link table? use this for generating many-to-many relations
        link: bool,

        /// Model fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    Migration {
        /// Name of the migration file
        name: String,

        /// Params fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Model and params fields, eg. title:string hits:int
        fields: Vec<(String, String)>,

        // k
        kind: ScaffoldKind,
    },
    Controller {
        /// Name of the thing to generate
        name: String,

        /// Action names
        actions: Vec<String>,

        // kind
        kind: ScaffoldKind,
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
    Deployment {
        kind: DeploymentKind,
        fallback_file: Option<String>,
        asset_folder: Option<String>,
        host: String,
        port: i32,
    },
}
pub struct AppInfo {
    pub app_name: String,
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
        Component::Model { name, link, fields } => {
            model::generate(rrgen, &name, link, &fields, appinfo)?
        }
        #[cfg(feature = "with-db")]
        Component::Scaffold { name, fields, kind } => {
            scaffold::generate(rrgen, &name, &fields, &kind, appinfo)?
        }
        #[cfg(feature = "with-db")]
        Component::Migration { name, fields } => {
            migration::generate(rrgen, &name, &fields, appinfo)?
        }
        Component::Controller {
            name,
            actions,
            kind,
        } => controller::generate(rrgen, &name, &actions, &kind, appinfo)?,
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
        Component::Deployment {
            kind,
            fallback_file,
            asset_folder,
            host,
            port,
        } => match kind {
            DeploymentKind::Docker => {
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "copy_asset_folder": asset_folder.unwrap_or_default(),
                    "fallback_file": fallback_file.unwrap_or_default()
                });
                render_template(rrgen, Path::new("deployment/docker"), &vars)?
            }
            DeploymentKind::Shuttle => {
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "shuttle_runtime_version": DEPLOYMENT_SHUTTLE_RUNTIME_VERSION,
                    "with_db": cfg!(feature = "with-db")
                });

                render_template(rrgen, Path::new("deployment/shuttle"), &vars)?
            }
            DeploymentKind::Nginx => {
                let host = host.replace("http://", "").replace("https://", "");
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "domain": host,
                    "port": port
                });
                render_template(rrgen, Path::new("deployment/nginx"), &vars)?
            }
        },
    };

    Ok(get_result)
}

fn render_template(rrgen: &RRgen, template: &Path, vars: &Value) -> Result<GenerateResults> {
    let template_files = template::collect_files_from_path(template)?;

    let mut gen_result = vec![];
    let mut local_templates = vec![];
    for template in template_files {
        let custom_template = Path::new(template::DEFAULT_LOCAL_TEMPLATE).join(template.path());

        if custom_template.exists() {
            let content = fs::read_to_string(&custom_template).map_err(|err| {
                tracing::error!(custom_template = %custom_template.display(), "could not read custom template");
                err
            })?;
            gen_result.push(rrgen.generate(&content, vars)?);
            local_templates.push(custom_template);
        } else {
            let content = template.contents_utf8().ok_or(Error::Message(format!(
                "could not get template content: {}",
                template.path().display()
            )))?;
            gen_result.push(rrgen.generate(content, vars)?);
        };
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
            messages.push_str(&format!("* {message}\n"));
        }
    }

    if !results.local_templates.is_empty() {
        messages.push_str(&format!(
            "{}",
            "\nThe following templates were sourced from the local templates:\n".green()
        ));
        for f in &results.local_templates {
            messages.push_str(&format!("* {}\n", f.display()));
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
                )))
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
}
