//! This task implements data seeding functionality for initializing new development/demo environments.
//!
//! # Example
//!
//! Run the task with the following command:
//! ```sh
//! cargo run task
//! ```
//!
//! To override existing data and reset the data structure, use the following command with the `refresh:true` argument:
//! ```sh
//! cargo run task seed_data refresh:true
//! ```
use std::collections::BTreeMap;

use crate::app::App;
use loco_rs::db;
use loco_rs::prelude::*;
use migration::Migrator;

#[allow(clippy::module_name_repetitions)]
pub struct SeedData;
#[async_trait]
impl Task for SeedData {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed_data".to_string(),
            detail: "Task for seeding data".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        let refresh = vars
            .get("refresh")
            .map(|refresh| refresh == "true")
            .unwrap_or(false);

        if refresh {
            db::reset::<Migrator>(&app_context.db).await?;
        }
        let path = std::path::Path::new("src/fixtures");
        db::run_app_seed::<App>(&app_context.db, path).await?;
        Ok(())
    }
}
