use loco_rs::{task, testing::prelude::*};
use {{settings.module_name}}::{app::App, models::users};

use loco_rs::boot::run_task;
use serial_test::serial;
use std::io::Cursor;

#[tokio::test]
#[serial]
async fn can_run_user_delete_by_email() {
    let boot = boot_test::<App>().await.unwrap();

    let email = "test@example.com";
    let user = users::Model::find_by_email(&boot.app_context.db, email).await;

    assert!(user.is_err());

    let user = users::Model::create_with_password(
        &boot.app_context.db,
        &users::RegisterParams {
            email: "test@example.com".to_string(),
            password: "securepassword".to_string(),
            name: "Test User".to_string(),
        }).await;

    assert!(user.is_ok());

    let vars = task::Vars::from_cli_args(vec![
        ("email".to_string(), email.to_string()),
    ]);

    assert!(
        run_task::<App>(&boot.app_context, Some(&"user:delete".to_string()), &vars)
            .await
            .is_ok()
    );

    let user = users::Model::find_by_email(&boot.app_context.db, email).await;   
    assert!(user.is_err());
    
}