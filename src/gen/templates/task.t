{% set file_name = name |  snake_case -%}
{% set struct_name = file_name | pascal_case -%}
to: "src/tasks/{{file_name}}.rs"
skip_glob: "src/tasks/{{file_name}}.rs"
message: "?????????????????."
injections:
- into: "src/tasks/mod.rs"
  append: true
  content: "pub mod {{ file_name }};"
- into: src/app.rs
  after: "fn register_tasks"
  content: "        tasks.register(tasks::{{file_name}}::{{struct_name}});"
---
use std::collections::BTreeMap;

use async_trait::async_trait;
use rustyrails::{
    app::AppContext,
    task::{Task, TaskInfo},
    Result,
};

pub struct {{struct_name}};
#[async_trait]
impl Task for {{struct_name}} {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "{{name}}".to_string(),
            detail: "Task generator".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        println!("Task {{struct_name}} generated");
        Ok(())
    }
}
