use std::collections::BTreeMap;

use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};

pub struct ExpReport;
#[async_trait]
impl Task for ExpReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "example".to_string(),
            detail: "output a example task".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &BTreeMap<String, String>) -> Result<()> {
        println!("example task executed");

        Ok(())
    }
}
