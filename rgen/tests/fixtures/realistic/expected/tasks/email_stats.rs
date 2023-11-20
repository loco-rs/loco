use std::collections::BTreeMap;

use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};
use sea_orm::EntityTrait;


pub struct EmailStats;

#[async_trait]
impl Task for EmailStats {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "email_stats".to_string(),
            detail: "a sample task".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &BTreeMap<String, String>) -> Result<()> {
        println!("hello email_stats");
        Ok(())
    }
}