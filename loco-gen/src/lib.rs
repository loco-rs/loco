// this is because not using with-db renders some of the structs below unused
// TODO: should be more properly aligned with extracting out the db-related gen
// code and then feature toggling it
#![allow(dead_code)]
use rrgen::{GenResult, RRgen};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod controller;
#[cfg(feature = "with-db")]
mod infer;
#[cfg(feature = "with-db")]
mod migration;
#[cfg(feature = "with-db")]
mod model;
#[cfg(feature = "with-db")]
mod scaffold;
#[cfg(test)]
mod testutil;
use std::{str::FromStr, sync::OnceLock};

const MAILER_T: &str = include_str!("templates/mailer/mailer.t");
const MAILER_SUB_T: &str = include_str!("templates/mailer/subject.t");
const MAILER_TEXT_T: &str = include_str!("templates/mailer/text.t");
const MAILER_HTML_T: &str = include_str!("templates/mailer/html.t");

const TASK_T: &str = include_str!("templates/task/task.t");
const TASK_TEST_T: &str = include_str!("templates/task/test.t");

const SCHEDULER_T: &str = include_str!("templates/scheduler/scheduler.t");

const WORKER_T: &str = include_str!("templates/worker/worker.t");
const WORKER_TEST_T: &str = include_str!("templates/worker/test.t");

// Deployment templates
const DEPLOYMENT_DOCKER_T: &str = include_str!("templates/deployment/docker/docker.t");
const DEPLOYMENT_DOCKER_IGNORE_T: &str = include_str!("templates/deployment/docker/ignore.t");
const DEPLOYMENT_SHUTTLE_T: &str = include_str!("templates/deployment/shuttle/shuttle.t");
const DEPLOYMENT_SHUTTLE_CONFIG_T: &str = include_str!("templates/deployment/shuttle/config.t");
const DEPLOYMENT_NGINX_T: &str = include_str!("templates/deployment/nginx/nginx.t");

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

#[derive(Debug, Clone)]
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
#[allow(clippy::too_many_lines)]
pub fn generate(component: Component, appinfo: &AppInfo) -> Result<()> {
    let rrgen = RRgen::default();
    /*
    (1)
    XXX: remove hooks generic from child generator, materialize it here and pass it
         means each generator accepts a [component, config, context] tuple
         this will allow us to test without an app instance
    (2) proceed to test individual generators
     */
    match component {
        #[cfg(feature = "with-db")]
        Component::Model { name, link, fields } => {
            println!(
                "{}",
                model::generate(&rrgen, &name, link, &fields, appinfo)?
            );
        }
        #[cfg(feature = "with-db")]
        Component::Scaffold { name, fields, kind } => {
            println!(
                "{}",
                scaffold::generate(&rrgen, &name, &fields, &kind, appinfo)?
            );
        }
        #[cfg(feature = "with-db")]
        Component::Migration { name, fields } => {
            migration::generate(&rrgen, &name, &fields, appinfo)?;
        }
        Component::Controller {
            name,
            actions,
            kind,
        } => {
            println!(
                "{}",
                controller::generate(&rrgen, &name, &actions, &kind, appinfo)?
            );
        }
        Component::Task { name } => {
            let vars = json!({"name": name, "pkg_name": appinfo.app_name});
            rrgen.generate(TASK_T, &vars)?;
            rrgen.generate(TASK_TEST_T, &vars)?;
        }
        Component::Scheduler {} => {
            let vars = json!({"pkg_name": appinfo.app_name});
            rrgen.generate(SCHEDULER_T, &vars)?;
        }
        Component::Worker { name } => {
            let vars = json!({"name": name, "pkg_name": appinfo.app_name});
            rrgen.generate(WORKER_T, &vars)?;
            rrgen.generate(WORKER_TEST_T, &vars)?;
        }
        Component::Mailer { name } => {
            let vars = json!({ "name": name });
            rrgen.generate(MAILER_T, &vars)?;
            rrgen.generate(MAILER_SUB_T, &vars)?;
            rrgen.generate(MAILER_TEXT_T, &vars)?;
            rrgen.generate(MAILER_HTML_T, &vars)?;
        }
        Component::Deployment {
            fallback_file,
            asset_folder,
            host,
            port,
        } => {
            let deployment_kind = match std::env::var("LOCO_DEPLOYMENT_KIND") {
                Ok(kind) => kind
                    .parse::<DeploymentKind>()
                    .map_err(|_e| Error::Message(format!("deployment {kind} not supported")))?,
                Err(_err) => prompt_deployment_selection().map_err(Box::from)?,
            };

            match deployment_kind {
                DeploymentKind::Docker => {
                    let vars = json!({
                        "pkg_name": appinfo.app_name,
                        "copy_asset_folder": asset_folder.unwrap_or_default(),
                        "fallback_file": fallback_file.unwrap_or_default()
                    });
                    rrgen.generate(DEPLOYMENT_DOCKER_T, &vars)?;
                    rrgen.generate(DEPLOYMENT_DOCKER_IGNORE_T, &vars)?;
                }
                DeploymentKind::Shuttle => {
                    let vars = json!({
                        "pkg_name": appinfo.app_name,
                        "shuttle_runtime_version": DEPLOYMENT_SHUTTLE_RUNTIME_VERSION,
                        "with_db": cfg!(feature = "with-db")
                    });
                    rrgen.generate(DEPLOYMENT_SHUTTLE_T, &vars)?;
                    rrgen.generate(DEPLOYMENT_SHUTTLE_CONFIG_T, &vars)?;
                }
                DeploymentKind::Nginx => {
                    let host = host.replace("http://", "").replace("https://", "");
                    let vars = json!({
                        "pkg_name": appinfo.app_name,
                        "domain": host,
                        "port": port
                    });
                    rrgen.generate(DEPLOYMENT_NGINX_T, &vars)?;
                }
            }
        }
    }
    Ok(())
}

fn collect_messages(results: Vec<GenResult>) -> String {
    let mut messages = String::new();
    for res in results {
        if let rrgen::GenResult::Generated {
            message: Some(message),
        } = res
        {
            messages.push_str(&format!("* {message}\n"));
        }
    }
    messages
}
use dialoguer::{theme::ColorfulTheme, Select};

fn prompt_deployment_selection() -> Result<DeploymentKind> {
    let options: Vec<String> = DEPLOYMENT_OPTIONS.iter().map(|t| t.0.to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("❯ Choose your deployment")
        .items(&options)
        .default(0)
        .interact()
        .map_err(Error::msg)?;

    Ok(DEPLOYMENT_OPTIONS[selection].1.clone())
}
