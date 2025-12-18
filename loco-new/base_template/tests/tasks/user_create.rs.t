use loco_rs::{task, testing::prelude::*};
use {{settings.module_name}}::{app::App, models::users};

use loco_rs::boot::run_task;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_run_user_create() {
    let boot = boot_test::<App>().await.unwrap();

    let email = "test@example.com";
    let user = users::Model::find_by_email(&boot.app_context.db, email).await;
    assert!(user.is_err());

    let vars = task::Vars::from_cli_args(vec![
        ("email".to_string(), email.to_string()),
        ("name".to_string(), "Test User".to_string()),
        ("password".to_string(), "securepassword".to_string()),
    ]);
    assert!(
        run_task::<App>(&boot.app_context, Some(&"user:create".to_string()), &vars)
            .await
            .is_ok()
    );

    let deliveries = boot.app_context.mailer.unwrap().deliveries();
    assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

    let user = users::Model::find_by_email(&boot.app_context.db, email).await;
    assert!(user.is_ok());
}

#[tokio::test]
#[serial]
async fn test_user_email_already_exists() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let email = "user1@example.com";

    let vars = task::Vars::from_cli_args(vec![
        ("email".to_string(), email.to_string()),
        ("name".to_string(), "Test User".to_string()),
        ("password".to_string(), "securepassword".to_string()),
    ]);
    let err = run_task::<App>(&boot.app_context, Some(&"user:create".to_string()), &vars)
        .await
        .expect_err("err");

    assert_eq!(
        err.to_string(),
        "Failed to create user. err: Entity already exists"
    );

    let deliveries = boot.app_context.mailer.unwrap().deliveries();
    assert_eq!(deliveries.count, 0, "No email should be sent");
}