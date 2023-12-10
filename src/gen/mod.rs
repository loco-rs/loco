use rrgen::{GenResult, RRgen};
use serde_json::json;

#[cfg(feature = "with-db")]
mod model;
#[cfg(feature = "with-db")]
mod scaffold;
use crate::{app::Hooks, errors, Result};
use std::str::FromStr;

const CONTROLLER_T: &str = include_str!("templates/controller.t");
const CONTROLLER_TEST_T: &str = include_str!("templates/request_test.t");

const MAILER_T: &str = include_str!("templates/mailer.t");
const MAILER_SUB_T: &str = include_str!("templates/mailer_sub.t");
const MAILER_TEXT_T: &str = include_str!("templates/mailer_text.t");
const MAILER_HTML_T: &str = include_str!("templates/mailer_html.t");

const TASK_T: &str = include_str!("templates/task.t");
const TASK_TEST_T: &str = include_str!("templates/task_test.t");

const WORKER_T: &str = include_str!("templates/worker.t");
const WORKER_TEST_T: &str = include_str!("templates/worker_test.t");

const DEPLOYMENT_DOCKER_T: &str = include_str!("templates/deployment_docker.t");

const DEPLOYMENT_OPTIONS: &[(&str, DeploymentKind)] = &[("Docker", DeploymentKind::Docker)];

#[derive(Debug, Clone)]
pub enum DeploymentKind {
    Docker,
}
impl FromStr for DeploymentKind {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(Self::Docker),
            _ => Err(()),
        }
    }
}

pub enum Component {
    #[cfg(feature = "with-db")]
    Model {
        /// Name of the thing to generate
        name: String,

        /// Model fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    #[cfg(feature = "with-db")]
    Scaffold {
        /// Name of the thing to generate
        name: String,

        /// Model and params fields, eg. title:string hits:int
        fields: Vec<(String, String)>,
    },
    Controller {
        /// Name of the thing to generate
        name: String,
    },
    Task {
        /// Name of the thing to generate
        name: String,
    },
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

pub fn generate<H: Hooks>(component: Component) -> Result<()> {
    let rrgen = RRgen::default();

    match component {
        #[cfg(feature = "with-db")]
        Component::Model { name, fields } => {
            println!("{}", model::generate::<H>(&rrgen, &name, &fields)?);
        }
        #[cfg(feature = "with-db")]
        Component::Scaffold { name, fields } => {
            println!("{}", scaffold::generate::<H>(&rrgen, &name, &fields)?);
        }
        Component::Controller { name } => {
            let vars = json!({"name": name});
            rrgen.generate(CONTROLLER_T, &vars)?;
            rrgen.generate(CONTROLLER_TEST_T, &vars)?;
        }
        Component::Task { name } => {
            let vars = json!({"name": name, "pkg_name": H::app_name()});

            rrgen.generate(TASK_T, &vars)?;
            rrgen.generate(TASK_TEST_T, &vars)?;
        }
        Component::Worker { name } => {
            let vars = json!({"name": name, "pkg_name": H::app_name()});

            rrgen.generate(WORKER_T, &vars)?;
            rrgen.generate(WORKER_TEST_T, &vars)?;
        }
        Component::Mailer { name } => {
            let vars = json!({"name": name});
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
            let vars = json!({ "pkg_name": H::app_name()});
            match deployment_kind {
                DeploymentKind::Docker => {
                    rrgen.generate(DEPLOYMENT_DOCKER_T, &vars)?;
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

fn prompt_deployment_selection() -> eyre::Result<DeploymentKind> {
    let options: Vec<String> = DEPLOYMENT_OPTIONS.iter().map(|t| t.0.to_string()).collect();

    let selection_options = requestty::Question::select("deployment")
        .message("❯ Choose your deployment")
        .choices(&options)
        .build();

    let answer = requestty::prompt_one(selection_options)?;

    let selection = answer
        .as_list_item()
        .ok_or_else(|| eyre::eyre!("deployment selection it empty"))?;

    Ok(DEPLOYMENT_OPTIONS[selection.index].1.clone())
}
