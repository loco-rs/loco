use chrono::{offset::Local, Duration};
use loco_rs::prelude::*;

use crate::models::users;

/// Reports how many users signed up in a recent window.
///
/// This is a `Task`, not a worker: nothing is waiting on an HTTP response, and
/// an operator needs to be able to run it by hand. `config/scheduler.yaml`
/// invokes this same task at 3 AM — the scheduler shells out to tasks rather
/// than holding logic of its own, so the nightly run and the manual run are one
/// implementation.
pub struct SignupDigest;

#[async_trait]
impl Task for SignupDigest {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "signup_digest".to_string(),
            detail: "Report how many users signed up in the last N hours.\nUsage:\ncargo loco task signup_digest hours:24".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        // `cli_arg` errors when the key is absent, so an optional argument
        // falls back rather than propagating.
        let hours: i64 = vars
            .cli_arg("hours")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(24);

        let since = Local::now() - Duration::hours(hours);
        let count = users::Model::count_registered_since(&ctx.db, since.into()).await?;

        tracing::info!(hours, count, "signup digest");
        Ok(())
    }
}
