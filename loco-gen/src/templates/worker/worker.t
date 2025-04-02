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
  content: "        queue.register(crate::workers::{{module_name}}::Worker::build(ctx)).await?;"---
use serde::{Deserialize, Serialize};
use loco_rs::prelude::*;

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, _args: WorkerArgs) -> Result<()> {
        println!("================={{struct_name}}=======================");
        // TODO: Some actual work goes here...
        Ok(())
    }
}
