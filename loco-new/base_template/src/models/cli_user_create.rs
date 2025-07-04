use dialoguer::{theme::ColorfulTheme, Input, Password};
use loco_rs::{
    app::AppContext,
    Result
};
use migration::{Migrator, MigratorTrait};
use super::users::Model;
use super::users::RegisterParams;

pub async fn create(ctx: &AppContext) -> Result<()> {

    let db = &ctx.db;

    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("👤 ❯ Enter username")
        .interact_text()?;

    let email: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("📧 Enter email")
        .interact_text()?;

    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("🔒 ❯ Enter password")
        .with_confirmation("⚠️ ❯ Confirm password", "Passwords don't match")
        .interact()?;

    let params = RegisterParams {
        name: name.trim().to_string(),
        email: email.trim().to_string(),
        password: password,
    };

    Migrator::up(db, None).await?;
    Model::create_with_password(db, &params).await?;
    println!("✅ User created successfully!");

    Ok(())
}
