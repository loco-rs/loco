{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: "src/tasks/{{file_name}}.rs"
skip_exists: true
message: "A Task `{{module_name}}` was added successfully. Run with `cargo run task {{name}}`."
injections:
- into: "src/tasks/mod.rs"
  append: true
  content: "pub mod {{ file_name }};"
- into: src/app.rs
  after: "fn register_tasks"
  content: "        tasks.register(tasks::{{file_name}}::{{module_name}});"
---
use std::collections::BTreeMap;

use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};

pub struct {{module_name}};
#[async_trait]
impl Task for {{module_name}} {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "{{name}}".to_string(),
            detail: "Task generator".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &BTreeMap<String, String>) -> Result<()> {
        println!("Task {{module_name}} generated");
        Ok(())
    }
}
