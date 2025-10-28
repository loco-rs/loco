use loco_rs::prelude::*;
use dialoguer::{theme::ColorfulTheme, Input, Confirm};

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
            detail: "Delete a user by entering email or PID.".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {

        let input = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("❯ Enter email or PID to delete user")
                .interact_text()?;
        
        let user_to_delete = users::Model::find_by_email(&app_context.db, &input)
                .await
                .ok_or_else(|| {
                    users::Model::find_by_pid(&app_context.db, &input)
                        .await
                        .ok_or_else(|err| {
                            tracing::error!(
                                message = "Failed to find user to delete",
                                );
                            Err(Error::string(&format!("Failed to find user to delete. err: {err}")))
                        })
                })?;
        
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("❓ Are you sure you want to delete this user?\n👤 {user_to_delete.name} ({user_to_delete.email})"))
                .interact()?;
        if !confirm {
            println!(message = "⛔ User deletion cancelled - nothing has been deleted!");
            return Ok(());
        }

        let _ = user_to_delete.into_active_model().delete(&app_context.db).await;
        println!("🗑️ User deleted successfully!");
        Ok(())
}
}