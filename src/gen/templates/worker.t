{% set module_name = name |  snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/workers/{{module_name}}.rs"
skip_exists: true
message: "A worker `{{struct_name}}` was added successfully. Run with `cargo run start --worker`."
injections:
- into: "src/workers/mod.rs"
  append: true
  content: "pub mod {{ module_name}};"
- into: src/app.rs
  after: "fn connect_workers"
  content: "        p.register(crate::workers::{{module_name}}::{{struct_name}}::build(ctx));"
---
use serde::{Deserialize, Serialize};
use loco_rs::prelude::*;

pub struct {{struct_name}} {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct {{struct_name}}Args {
    pub user_guid: String,
}

impl worker::AppWorker<{{struct_name}}Args> for {{struct_name}} {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}

#[async_trait]
impl worker::Worker<{{struct_name}}Args> for {{struct_name}} {
    async fn perform(&self, _args: {{struct_name}}Args) -> worker::Result<()> {
        println!("================={{struct_name}}=======================");
        // TODO: Some actual work goes here...
        Ok(())
    }
}
