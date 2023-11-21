use chrono::Utc;
use rrgen::RRgen;
use serde_json::json;

use crate::Result;
const CONTROLLER_T: &str = include_str!("templates/controller.t");
const CONTROLLER_TEST_T: &str = include_str!("templates/request_test.t");

const MODEL_T: &str = include_str!("templates/model.t");
const MODEL_TEST_T: &str = include_str!("templates/model_test.t");

const MAILER_T: &str = include_str!("templates/mailer.t");
const MAILER_SUB_T: &str = include_str!("templates/mailer_sub.t");
const MAILER_TEXT_T: &str = include_str!("templates/mailer_text.t");
const MAILER_HTML_T: &str = include_str!("templates/mailer_html.t");

const TASK_T: &str = include_str!("templates/task.t");

const WORKER_T: &str = include_str!("templates/worker.t");

pub enum Component {
    Model {
        /// Name of the thing to generate
        name: String,

        /// Model fields, eg. title=string hits=integer (unimplemented)
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
}
pub fn generate(component: Component) -> Result<()> {
    let rrgen = RRgen::default();
    let pkg_name = env!("CARGO_PKG_NAME");
    let ts = Utc::now();

    match component {
        Component::Model { name, fields: _ } => {
            let vars = json!({"name": name, "ts": ts, "pkg_name": pkg_name});
            rrgen.generate(MODEL_T, &vars)?;
            rrgen.generate(MODEL_TEST_T, &vars)?;
        }
        Component::Controller { name } => {
            let vars = json!({"name": name});
            rrgen.generate(CONTROLLER_T, &vars)?;
            rrgen.generate(CONTROLLER_TEST_T, &vars)?;
        }
        Component::Task { name } => {
            let vars = json!({"name": name});
            rrgen.generate(TASK_T, &vars)?;
        }
        Component::Worker { name } => {
            let vars = json!({"name": name});
            rrgen.generate(WORKER_T, &vars)?;
        }
        Component::Mailer { name } => {
            let vars = json!({"name": name});
            rrgen.generate(MAILER_T, &vars)?;
            rrgen.generate(MAILER_SUB_T, &vars)?;
            rrgen.generate(MAILER_TEXT_T, &vars)?;
            rrgen.generate(MAILER_HTML_T, &vars)?;
        }
    }
    Ok(())
}
