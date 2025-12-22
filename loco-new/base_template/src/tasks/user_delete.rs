use crate::models::_entities::users;
use loco_rs::prelude::*;
use std::env;

pub struct UserDelete;
#[async_trait]
impl Task for UserDelete {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user:delete".to_string(),
            detail: "Delete a user by entering email.\nUsage:\ncargo run task user:delete"
                .to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let input = match vars.cli_arg("pid") {
            Ok(email) => email,
            Err(_) => return Err(Error::string("pid is mandatory")),
        };

        let user_to_delete = users::Model::find_by_pid(&app_context.db, &input).await?;

        tracing::info!(
            message = "User to delete",
            user_email = &user_to_delete.email,
            user_pid = user_to_delete.pid.to_string(),
            "delete user via task"
        );

        let mut confirm = String::new();

        // If test ist running simulate confirmation
        if let Ok(var) = env::var("TEST_CAN_RUN_USER_DELET_BY_PID") {
            if var == "true".to_string() {
                confirm.push_str("yes");
            }
        } else {
            println!(
                "Are you sure you want to delete the user {}\n({})\nwith pid '{}'?\nType 'yes' and hit enter to confirm",
                user_to_delete.name, user_to_delete.email, user_to_delete.pid
            );
            let stdin = std::io::stdin();
            stdin.read_line(&mut confirm).map_err(|err| {
                tracing::error!(
                    message = err.to_string(),
                    "could not read confirmation input"
                );
                Error::string(&format!("Failed to read confirmation input. err: {err}",))
            })?;
        }

        if confirm.trim().to_lowercase() != "yes" {
            println!("⛔ User deletion cancelled - nothing has been deleted!");
            return Ok(());
        }

        let _deleted_user = user_to_delete
            .into_active_model()
            .delete(&app_context.db)
            .await
            .map_err(|err| {
                tracing::error!(message = err.to_string(), "could not delete user");
                Error::string(&format!("Failed to delete user. err: {err}",))
            })?;
        println!("🗑️ User deleted successfully!");

        Ok(())
    }
}
