// this is because not using with-db renders some of the structs below unused
// TODO: should be more properly aligned with extracting out the db-related gen
// code and then feature toggling it
#![allow(dead_code)]
use lazy_static::lazy_static;
use rrgen::{GenResult, RRgen};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod controller;
#[cfg(feature = "with-db")]
mod model;
#[cfg(feature = "with-db")]
mod scaffold;
use std::str::FromStr;

use crate::{app::Hooks, config::Config, errors, Result};

const CONTROLLER_T: &str = include_str!("templates/controller.t");
const CONTROLLER_TEST_T: &str = include_str!("templates/request_test.t");

const MAILER_T: &str = include_str!("templates/mailer.t");
const MAILER_SUB_T: &str = include_str!("templates/mailer_sub.t");
const MAILER_TEXT_T: &str = include_str!("templates/mailer_text.t");
const MAILER_HTML_T: &str = include_str!("templates/mailer_html.t");

const MIGRATION_T: &str = include_str!("templates/migration.t");

const TASK_T: &str = include_str!("templates/task.t");
const TASK_TEST_T: &str = include_str!("templates/task_test.t");

const SCHEDULER_T: &str = include_str!("templates/scheduler.t");

const WORKER_T: &str = include_str!("templates/worker.t");
const WORKER_TEST_T: &str = include_str!("templates/worker_test.t");

// Deployment templates
const DEPLOYMENT_DOCKER_T: &str = include_str!("templates/deployment_docker.t");
const DEPLOYMENT_DOCKER_IGNORE_T: &str = include_str!("templates/deployment_docker_ignore.t");
const DEPLOYMENT_SHUTTLE_T: &str = include_str!("templates/deployment_shuttle.t");
const DEPLOYMENT_SHUTTLE_CONFIG_T: &str = include_str!("templates/deployment_shuttle_config.t");
const DEPLOYMENT_NGINX_T: &str = include_str!("templates/deployment_nginx.t");

const DEPLOYMENT_SHUTTLE_RUNTIME_VERSION: &str = "0.46.0";

const DEPLOYMENT_OPTIONS: &[(&str, DeploymentKind)] = &[
    ("Docker", DeploymentKind::Docker),
    ("Shuttle", DeploymentKind::Shuttle),
    ("Nginx", DeploymentKind::Nginx),
];

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

lazy_static! {
    static ref MAPPINGS: Mappings = {
        let json_data = include_str!("./mappings.json");
        serde_json::from_str(json_data).expect("JSON was not well-formatted")
    };
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

pub enum Component {
    #[cfg(feature = "with-db")]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Is it a link table? use this for generating many-to-many relations
        link: bool,

        /// Model fields, eg. title:string hits:int
        fields: Vec<(String, String)>,

        /// Generate migration code and stop, don't run the migration
        migration_only: bool,
    },
    #[cfg(feature = "with-db")]
    Migration {
        /// Name of the migration file
        name: String,
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
    Deployment {},
}
#[allow(clippy::too_many_lines)]
pub fn generate<H: Hooks>(component: Component, config: &Config) -> Result<()> {
    let rrgen = RRgen::default();
    match component {
        #[cfg(feature = "with-db")]
        Component::Model {
            name,
            link,
            fields,
            migration_only,
        } => {
            println!(
                "{}",
                model::generate::<H>(&rrgen, &name, link, migration_only, &fields)?
            );
        }
        #[cfg(feature = "with-db")]
        Component::Scaffold { name, fields, kind } => {
            println!(
                "{}",
                scaffold::generate::<H>(&rrgen, &name, &fields, &kind)?
            );
        }
        #[cfg(feature = "with-db")]
        Component::Migration { name } => {
            let vars = json!({ "name": name, "ts": chrono::Utc::now(), "pkg_name": H::app_name()});
            rrgen.generate(MIGRATION_T, &vars)?;
        }
        Component::Controller {
            name,
            actions,
            kind,
        } => {
            println!(
                "{}",
                controller::generate::<H>(&rrgen, &name, &actions, &kind)?
            );
        }
        Component::Task { name } => {
            let vars = json!({"name": name, "pkg_name": H::app_name()});
            rrgen.generate(TASK_T, &vars)?;
            rrgen.generate(TASK_TEST_T, &vars)?;
        }
        Component::Scheduler {} => {
            let vars = json!({"pkg_name": H::app_name()});
            rrgen.generate(SCHEDULER_T, &vars)?;
        }
        Component::Worker { name } => {
            let vars = json!({"name": name, "pkg_name": H::app_name()});
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
        Component::Deployment {} => {
            let deployment_kind = match std::env::var("LOCO_DEPLOYMENT_KIND") {
                Ok(kind) => kind.parse::<DeploymentKind>().map_err(|_e| {
                    errors::Error::Message(format!("deployment {kind} not supported"))
                })?,
                Err(_err) => prompt_deployment_selection().map_err(Box::from)?,
            };

            match deployment_kind {
                DeploymentKind::Docker => {
                    let copy_asset_folder =
                        &config.server.middlewares.static_assets.folder.path.clone();

                    let fallback_file = &config.server.middlewares.static_assets.fallback.clone();

                    let vars = json!({
                        "pkg_name": H::app_name(),
                        "copy_asset_folder": copy_asset_folder,
                        "fallback_file": fallback_file
                    });
                    rrgen.generate(DEPLOYMENT_DOCKER_T, &vars)?;
                    rrgen.generate(DEPLOYMENT_DOCKER_IGNORE_T, &vars)?;
                }
                DeploymentKind::Shuttle => {
                    let vars = json!({
                        "pkg_name": H::app_name(),
                        "shuttle_runtime_version": DEPLOYMENT_SHUTTLE_RUNTIME_VERSION,
                        "with_db": cfg!(feature = "with-db")
                    });
                    rrgen.generate(DEPLOYMENT_SHUTTLE_T, &vars)?;
                    rrgen.generate(DEPLOYMENT_SHUTTLE_CONFIG_T, &vars)?;
                }
                DeploymentKind::Nginx => {
                    let host = &config
                        .server
                        .host
                        .replace("http://", "")
                        .replace("https://", "");
                    let vars = json!({
                        "pkg_name": H::app_name(),
                        "domain": &host,
                        "port": &config.server.port
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
        .map_err(errors::Error::msg)?;

    Ok(DEPLOYMENT_OPTIONS[selection].1.clone())
}
