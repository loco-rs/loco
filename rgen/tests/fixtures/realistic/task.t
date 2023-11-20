---
to: tests/fixtures/realistic/generated/tasks/{{name | snake_case }}.rs
injections:
- into: tests/fixtures/realistic/generated/tasks/mod.rs
  append: true
  content: "pub mod {{ name | snake_case }};"
- into: tests/fixtures/realistic/generated/app.rs
  after: "fn register_tasks"
  content: "        tasks.register(tasks::{{ name | snake_case }}::{{ name | pascal_case }});"
---
use std::collections::BTreeMap;

use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};
use sea_orm::EntityTrait;


pub struct {{ name | pascal_case }};

#[async_trait]
impl Task for {{ name | pascal_case }} {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "{{ name }}".to_string(),
            detail: "a sample task".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &BTreeMap<String, String>) -> Result<()> {
        println!("hello {{ name }}");
        Ok(())
    }
}
