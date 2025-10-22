use loco_rs::prelude::*;

use crate::{
    mailers::auth::AuthMailer,
    models::{_entities::users, users::RegisterParams},
};

pub struct UserCreate;
#[async_trait]
impl Task for UserCreate {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user:create".to_string(),
            detail: "Create a new user with email, name, and password. Sends welcome email and sets up email verification.\nUsage:\ncargo run task user:create email:user@example.com name:\"John Doe\" password:\"securepassword\"".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let email = vars
            .cli_arg("email")
            .map_err(|_| Error::string("email is mandatory"))?;
        let name = vars
            .cli_arg("name")
            .map_err(|_| Error::string("name is mandatory"))?;
        let password = vars
            .cli_arg("password")
            .map_err(|_| Error::string("password is mandatory"))?;

        let register_params = RegisterParams {
            email: email.to_string(),
            password: password.to_string(),
            name: name.to_string(),
        };

        // Create user with password using the same logic as register controller
        let res = users::Model::create_with_password(&app_context.db, &register_params).await;

        let user = match res {
            Ok(user) => {
                tracing::info!(
                    message = "User created successfully",
                    user_email = &register_params.email,
                    user_pid = user.pid.to_string(),
                    "user created via task"
                );
                user
            }
            Err(err) => {
                tracing::error!(
                    message = err.to_string(),
                    user_email = &register_params.email,
                    "could not create user via task"
                );
                return Err(Error::string(
                    &format!("Failed to create user. err: {err}",),
                ));
            }
        };

        // Set email verification sent (same as register controller)
        let user = user
            .into_active_model()
            .set_email_verification_sent(&app_context.db)
            .await
            .map_err(|err| {
                tracing::error!(
                    message = err.to_string(),
                    user_email = &register_params.email,
                    "could not set email verification"
                );
                Error::string("Failed to set email verification")
            })?;

        // Send welcome email (same as register controller)
        AuthMailer::send_welcome(app_context, &user)
            .await
            .map_err(|err| {
                tracing::error!(
                    message = err.to_string(),
                    user_email = &register_params.email,
                    "could not send welcome email"
                );
                Error::string("Failed to send welcome email")
            })?;

        tracing::info!(
            message = "User creation task completed successfully",
            user_email = &register_params.email,
            user_pid = user.pid.to_string(),
            "user creation task finished"
        );

        println!("✅ User created successfully!");
        println!("   Email: {}", user.email);
        println!("   Name: {}", user.name);
        println!("   PID: {}", user.pid);

        Ok(())
    }
}
