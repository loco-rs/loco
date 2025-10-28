use loco_rs::{task, testing::prelude::*};
use {{settings.module_name}}::{app::App, models::users};

use loco_rs::boot::run_task;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn ca_run_user_delete() {
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
        }).await.unwrap();


    let user_pid = user.pid.to_string();

    // TODO: FINISH TEST FUNCTION
    
}