use loco_rs::prelude::*;
use dialoguer::{theme::ColorfulTheme, Confirm};

use crate::{
    mailers::auth::AuthMailer,
    models::{_entities::users, users::RegisterParams},
};

pub struct UserDelete;
#[async_trait]
impl Task for UserDelete {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user:delete".to_string(),
            detail: "Delete a user by entering email.\nUsage:\ncargo run task user:delete".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {

        let input = match vars.cli_arg("email") {
            Ok(email) => email,
            Err(_) => return Err(Error::string("email is mandatory")),
        };
        
        let user_to_delete = users::Model::find_by_email(&app_context.db, &input).await;

        match user_to_delete {
            Ok(user) => { tracing::info!(
                message = "User to delete",
                user_email = &user.email,
                user_pid = user.pid.to_string(),
                "delete user via task"
                )
            },
            Err(err) => {
                tracing::error!(
                    message = err.to_string(),
                    user_email = &register_params.email,
                    "could not delete user via task (user not found)"
                );
                return Err(Error::string(
                    &format!("Failed to delete user. User not found. err: {err}",),
                ));
            },
        }
        
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("❓ Are you sure you want to delete this user?\n👤 {user_to_delete.name} ({user_to_delete.email})"))
                .interact()?;
        if !confirm {
            println!("⛔ User deletion cancelled - nothing has been deleted!");
            return Ok(());
        }

        let _deleted_user = user_to_delete.into_active_model().delete(&app_context.db).await;
        println!("🗑️ User deleted successfully!");
        Ok(())
}
}