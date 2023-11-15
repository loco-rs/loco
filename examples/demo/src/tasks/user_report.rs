use std::collections::BTreeMap;

use async_trait::async_trait;
use framework::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};
use sea_orm::EntityTrait;

use crate::models::_entities::users;

pub struct UserReport;
#[async_trait]
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_report".to_string(),
            detail: "output a user report".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        let users = users::Entity::find().all(&app_context.db).await?;
        println!("args: {vars:?}");
        println!("!!! user_report: listing users !!!");
        println!("------------------------");
        for user in &users {
            println!("user: {}", user.email);
        }
        println!("done: {} users", users.len());
        Ok(())
    }
}
