use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};

pub const MIGRATION_SRC_LIB: &str = r"
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
        ";

pub const APP_ROUTS: &str = r"
impl Hooks for App {
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
        }
    }
";

pub const APP_TASK: &str = r"
impl Hooks for App {
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
";

pub const APP_WORKER: &str = r"
async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
    queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }
";

pub fn guess_file_by_time(path: &Path, file_format: &str, max_attempts: u32) -> Option<PathBuf> {
    let now = Utc::now();

    for seconds_to_subtract in 0..=max_attempts {
        let guessed_time = now - Duration::seconds(i64::from(seconds_to_subtract));
        let formatted_time = guessed_time.format("%Y%m%d_%H%M%S").to_string();
        let file_name = file_format.replace("{TIME}", &formatted_time);

        let file_path = path.join(file_name);
        if file_path.exists() {
            return Some(file_path);
        }
    }

    None
}
