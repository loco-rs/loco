use dialoguer::{Input, Password};
use {{settings.module_name}}::app::App;
use {{settings.module_name}}::models::user::Model;
use models::users::{RegisterParams, ModelResult};
use std::env;


#[tokio::main]
pub async fn main() -> loco_rs::Result<()> {

    env::set_var("RUN_MODE", "development");

    let app = App::build().await?;
    let ctx = Context::root().with_app(&app);
    let db = app.db().await?;


    let name = Input::new()
        .with_prompt("👤 Enter username")
        .interact_text()?;
    
    let email = Input::new()
        .with_prompt("📧 Enter email")
        .interact_text()?;
    
    let password = Password::new()
        .with_prompt("🔒 Enter password")
        .with_confirmation("⚠️ Confirm password", "Passwords don't match")
        .interact()?;
    
    let params = RegisterParams {
        name,
        email,
        password: password,
    };
    
    Model::create_with_password(db, &params).await?;
    println!("✅ User created successfully!");
    Ok(())
}
