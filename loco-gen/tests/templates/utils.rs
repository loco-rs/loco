use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};

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

/// A minimal `frontend/src/routes.tsx` fixture mirroring the shape of
/// `examples/reference_spa/frontend/src/routes.tsx`, but carrying the
/// `// scaffold:imports` / `// scaffold:routes` anchor comments the Api
/// scaffold's `frontend_list.t` injects into. The once-per-app base
/// `routes.tsx` (workstream 2c) must ship these two markers for real apps.
pub const ROUTES_TSX_FIXTURE: &str = r"import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { RequireAuth } from './auth/RequireAuth'
import { Home } from './pages/Home'
// scaffold:imports

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { index: true, element: <Home /> },
      { path: 'login', element: <Login /> },
      {
        element: <RequireAuth />,
        children: [
          // scaffold:routes
        ],
      },
    ],
  },
])
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
