// this is because not using with-db renders some of the structs below unused
// TODO: should be more properly aligned with extracting out the db-related gen
// code and then feature toggling it
#![allow(dead_code)]
pub use rrgen::{GenResult, RRgen};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
mod controller;
use colored::Colorize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[cfg(feature = "with-db")]
mod infer;
#[cfg(feature = "with-db")]
mod migration;
#[cfg(feature = "with-db")]
mod model;
#[cfg(feature = "with-db")]
mod scaffold;
pub mod template;
pub mod tera_ext;
#[cfg(test)]
mod testutil;

#[derive(Debug)]
pub struct GenerateResults {
    rrgen: Vec<rrgen::GenResult>,
    local_templates: Vec<PathBuf>,
}
const DEPLOYMENT_SHUTTLE_RUNTIME_VERSION: &str = "0.51.0";

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
    rust: RustType,
    schema: String,
    col_type: String,
    #[serde(default)]
    arity: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RustType {
    String(String),
    Map(HashMap<String, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mappings {
    field_types: Vec<FieldType>,
}
impl Mappings {
    fn error_unrecognized_default_field(&self, field: &str) -> Error {
        Self::error_unrecognized(field, &self.all_names())
    }

    fn error_unrecognized(field: &str, allow_fields: &[&String]) -> Error {
        Error::Message(format!(
            "type: `{}` not found. try any of: `{}`",
            field,
            allow_fields
                .iter()
                .map(|&s| s.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ))
    }

    /// Resolves the Rust type for a given field with optional parameters.
    ///
    /// # Errors
    ///
    /// if rust field not exists or invalid parameters
    pub fn rust_field_with_params(&self, field: &str, params: &Vec<String>) -> Result<&str> {
        match field {
            "array" | "array^" | "array!" => {
                if let RustType::Map(ref map) = self.rust_field_kind(field)? {
                    if let [single] = params.as_slice() {
                        let keys: Vec<&String> = map.keys().collect();
                        Ok(map
                            .get(single)
                            .ok_or_else(|| Self::error_unrecognized(field, &keys))?)
                    } else {
                        Err(self.error_unrecognized_default_field(field))
                    }
                } else {
                    Err(Error::Message(
                        "array field should configured as array".to_owned(),
                    ))
                }
            }

            _ => self.rust_field(field),
        }
    }

    /// Resolves the Rust type for a given field.
    ///
    /// # Errors
    ///
    /// When the given field not recognized
    pub fn rust_field_kind(&self, field: &str) -> Result<&RustType> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| &f.rust)
            .ok_or_else(|| self.error_unrecognized_default_field(field))
    }

    /// Resolves the Rust type for a given field.
    ///
    /// # Errors
    ///
    /// When the given field not recognized
    pub fn rust_field(&self, field: &str) -> Result<&str> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| &f.rust)
            .ok_or_else(|| self.error_unrecognized_default_field(field))
            .and_then(|rust_type| match rust_type {
                RustType::String(s) => Ok(s),
                RustType::Map(_) => Err(Error::Message(format!(
                    "type `{field}` need params to get the rust field type"
                ))),
            })
            .map(std::string::String::as_str)
    }

    /// Retrieves the schema field associated with the given field.
    ///
    /// # Errors
    ///
    /// When the given field not recognized
    pub fn schema_field(&self, field: &str) -> Result<&str> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.schema.as_str())
            .ok_or_else(|| self.error_unrecognized_default_field(field))
    }

    /// Retrieves the column type field associated with the given field.
    ///
    /// # Errors
    ///
    /// When the given field not recognized
    pub fn col_type_field(&self, field: &str) -> Result<&str> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.col_type.as_str())
            .ok_or_else(|| self.error_unrecognized_default_field(field))
    }

    /// Retrieves the column type arity associated with the given field.
    ///
    /// # Errors
    ///
    /// When the given field not recognized
    pub fn col_type_arity(&self, field: &str) -> Result<usize> {
        self.field_types
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.arity)
            .ok_or_else(|| self.error_unrecognized_default_field(field))
    }

    #[must_use]
    pub fn all_names(&self) -> Vec<&String> {
        self.field_types.iter().map(|f| &f.name).collect::<Vec<_>>()
    }
}

static MAPPINGS: OnceLock<Mappings> = OnceLock::new();

/// Get type mapping for generation
///
/// # Panics
///
/// Panics if loading fails
pub fn get_mappings() -> &'static Mappings {
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

#[derive(Debug, Clone)]
pub enum DeploymentKind {
    Docker {
        copy_paths: Vec<PathBuf>,
        is_client_side_rendering: bool,
    },
    Shuttle {
        runttime_version: Option<String>,
    },
    Nginx {
        host: String,
        port: i32,
    },
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
    },
}

pub struct AppInfo {
    pub app_name: String,
}

#[must_use]
pub fn new_generator() -> RRgen {
    RRgen::default().add_template_engine(tera_ext::new())
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
            DeploymentKind::Shuttle { runttime_version } => {
                let vars = json!({
                    "pkg_name": appinfo.app_name,
                    "shuttle_runtime_version": runttime_version.unwrap_or_else( || DEPLOYMENT_SHUTTLE_RUNTIME_VERSION.to_string()),
                    "with_db": cfg!(feature = "with-db")
                });

                render_template(rrgen, Path::new("deployment/shuttle"), &vars)?
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
    use super::*;
    use std::path::Path;

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

    fn test_mapping() -> Mappings {
        Mappings {
            field_types: vec![
                FieldType {
                    name: "array".to_string(),
                    rust: RustType::Map(HashMap::from([
                        ("string".to_string(), "Vec<String>".to_string()),
                        ("chat".to_string(), "Vec<String>".to_string()),
                        ("int".to_string(), "Vec<i32>".to_string()),
                    ])),
                    schema: "array".to_string(),
                    col_type: "array_null".to_string(),
                    arity: 1,
                },
                FieldType {
                    name: "string^".to_string(),
                    rust: RustType::String("String".to_string()),
                    schema: "string_uniq".to_string(),
                    col_type: "StringUniq".to_string(),
                    arity: 0,
                },
            ],
        }
    }

    #[test]
    fn can_get_all_names_from_mapping() {
        let mapping = test_mapping();
        assert_eq!(
            mapping.all_names(),
            Vec::from([&"array".to_string(), &"string^".to_string()])
        );
    }

    #[test]
    fn can_get_col_type_arity_from_mapping() {
        let mapping = test_mapping();

        assert_eq!(mapping.col_type_arity("array").expect("Get array arity"), 1);
        assert_eq!(
            mapping
                .col_type_arity("string^")
                .expect("Get string^ arity"),
            0
        );

        assert!(mapping.col_type_arity("unknown").is_err());
    }

    #[test]
    fn can_get_col_type_field_from_mapping() {
        let mapping = test_mapping();

        assert_eq!(
            mapping.col_type_field("array").expect("Get array field"),
            "array_null"
        );

        assert!(mapping.col_type_field("unknown").is_err());
    }

    #[test]
    fn can_get_schema_field_from_mapping() {
        let mapping = test_mapping();

        assert_eq!(
            mapping.schema_field("string^").expect("Get string^ schema"),
            "string_uniq"
        );

        assert!(mapping.schema_field("unknown").is_err());
    }

    #[test]
    fn can_get_rust_field_from_mapping() {
        let mapping = test_mapping();

        assert_eq!(
            mapping
                .rust_field("string^")
                .expect("Get string^ rust field"),
            "String"
        );

        assert!(mapping.rust_field("array").is_err());

        assert!(mapping.rust_field("unknown").is_err(),);
    }

    #[test]
    fn can_get_rust_field_kind_from_mapping() {
        let mapping = test_mapping();

        assert!(mapping.rust_field_kind("string^").is_ok());

        assert!(mapping.rust_field_kind("unknown").is_err(),);
    }

    #[test]
    fn can_get_rust_field_with_params_from_mapping() {
        let mapping = test_mapping();

        assert_eq!(
            mapping
                .rust_field_with_params("string^", &vec!["string".to_string()])
                .expect("Get string^ rust field"),
            "String"
        );

        assert_eq!(
            mapping
                .rust_field_with_params("array", &vec!["string".to_string()])
                .expect("Get string^ rust field"),
            "Vec<String>"
        );
        assert!(mapping
            .rust_field_with_params("array", &vec!["unknown".to_string()])
            .is_err());

        assert!(mapping.rust_field_with_params("unknown", &vec![]).is_err());
    }
}
