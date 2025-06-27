use crate::app::AppContext;
use crate::model::user::ActiveModel;
use dialoguer::{Input, Password};
use sea_orm::Set;
use crate::hash::hash_password;
use crate::Result;

pub async fn run(ctx: &AppContext) -> Result<()> {
    let name = Input::<String>::new().with_prompt("👤 Enter username: ").interact_text()?;

    let email = Input::<String>::new()
        .with_prompt("📧 Enter e-mail address: ")
        .interact_text()?;

    let password = Password::new()
        .with_prompt("🔒 Enter password: ")
        .with_confirmation("...Confirm password: ", "⚠️ Error: passwords do not match")
        .interact()?;

        let hashed = hash_password(&password)?;

        let mut user = ActiveModel {
            name: Set(Some(name.clone())),
            email: Set(Some(email.clone())),
            password_hash: Set(Some(hashed)),
            ..Default::default()
        };
    
        let record = user.insert(&ctx.db).await?;
        println!("✅ User '{}' created with id {}", name, record.id);
        Ok(())
}
