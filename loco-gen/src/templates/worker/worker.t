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
    /// Creates a new instance of the Worker with the given application context.
    /// 
    /// This function is called when registering the worker with the queue system.
    /// 
    /// # Parameters
    /// * `ctx` - The application context containing shared resources
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    /// Returns the class name of the worker.
    /// 
    /// This name is used when enqueueing jobs and identifying the worker in logs.
    /// The implementation returns the struct name as a string.
    fn class_name() -> String {
        "{{struct_name}}".to_string()
    }

    /// Returns tags associated with this worker.
    /// 
    /// Tags can be used to filter which workers run during startup.
    /// The default implementation returns an empty vector (no tags).
    fn tags() -> Vec<String> {
        Vec::new()
    }
    
    /// Performs the actual work when a job is processed.
    /// 
    /// This is the main function that contains the worker's logic.
    /// It gets executed when a job is dequeued from the job queue.
    /// 
    /// # Returns
    /// * `Result<()>` - Ok if the job completed successfully, Err otherwise
    async fn perform(&self, _args: WorkerArgs) -> Result<()> {
        println!("================={{struct_name}}=======================");
        // TODO: Some actual work goes here...
        Ok(())
    }
}
