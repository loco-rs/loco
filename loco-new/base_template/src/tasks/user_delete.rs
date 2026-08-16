use crate::models::_entities::users;
use loco_rs::prelude::*;

pub struct UserDelete;
#[async_trait]
impl Task for UserDelete {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user:delete".to_string(),
            detail: "Delete a user by pid. Prompts for confirmation unless force:true is given.\nUsage:\ncargo run task user:delete pid:01H0... force:true".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let Ok(input) = vars.cli_arg("pid") else {
            return Err(Error::string("pid is mandatory"));
        };
        let force_flag = vars
            .cli_arg("force")
            .is_ok_and(|v| v.trim().to_lowercase() == "true");

        let user_to_delete = users::Model::find_by_pid(&app_context.db, input).await?;

        println!(
            "User to delete:\nUsername: {}\nEmail: {}\nPID: {}",
            user_to_delete.name, user_to_delete.email, user_to_delete.pid
        );

        if !force_flag {
            println!(
                "Are you sure you want to delete the user {}\n({})\nwith pid '{}'?\nType 'yes' and hit enter to confirm",
                user_to_delete.name, user_to_delete.email, user_to_delete.pid
            );
            // Read on a blocking thread: this is an async context, and stdin
            // blocks until the user types. Doing it inline stalls the runtime.
            let confirm = tokio::task::spawn_blocking(|| {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).map(|_| line)
            })
            .await
            .map_err(|err| Error::string(&format!("Failed to read confirmation. err: {err}")))?
            .map_err(|err| {
                tracing::error!(
                    message = err.to_string(),
                    "could not read confirmation input"
                );
                Error::string(&format!("Failed to read confirmation input. err: {err}"))
            })?;

            if confirm.trim().to_lowercase() != "yes" {
                println!("User deletion cancelled - nothing has been deleted!");
                return Ok(());
            }
        }

        let user_name = user_to_delete.name.clone();
        let user_email = user_to_delete.email.clone();
        let user_pid = user_to_delete.pid;

        user_to_delete
            .into_active_model()
            .delete(&app_context.db)
            .await
            .map_err(|err| {
                tracing::error!(message = err.to_string(), "could not delete user");
                Error::string(&format!("Failed to delete user. err: {err}"))
            })?;
        println!("User deleted successfully!");
        tracing::info!(
            pid = user_pid.to_string(),
            username = user_name,
            email = user_email,
            "User deleted"
        );

        Ok(())
    }
}
