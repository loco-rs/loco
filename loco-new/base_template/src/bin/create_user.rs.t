use dialoguer::{Input, Password, theme::ColorfulTheme};
use {{settings.module_name}}::models::users::Model;
use {{settings.module_name}}::models::users::RegisterParams;
use std::path::PathBuf;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use loco_rs::{environment::Environment, Result};


#[tokio::main]
pub async fn main() -> Result<()> {

    let config = Environment::load_from_folder(&Environment::Development, &PathBuf::from("config"))?;
    let db_url = config.database.uri;

    let db = Database::connect(db_url).await?;

    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("👤 ❯ Enter username")
        .interact_text()?;

    let email: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("📧 Enter email").interact_text()?;

    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("🔒 ❯ Enter password")
        .with_confirmation("⚠️ ❯ Confirm password", "Passwords don't match")
        .interact()?;

    let params = RegisterParams {
        name: name.trim().to_string(),
        email: email.trim().to_string(),
        password: password,
    };
    
    Migrator::up(&db, None).await?;
    Model::create_with_password(&db, &params).await?;
    println!("✅ User created successfully!");

    Ok(())
}
