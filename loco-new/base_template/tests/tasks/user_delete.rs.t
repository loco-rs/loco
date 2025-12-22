use loco_rs::{task, testing::prelude::*};
use {{settings.module_name}}::{app::App, models::users};
use std::env;

use loco_rs::boot::run_task;
use serial_test::serial;


#[tokio::test]
#[serial]
async fn can_run_user_delete_by_pid() {
    env::set_var("TEST_CAN_RUN_USER_DELET_BY_PID", "true");
    let boot = boot_test::<App>().await.unwrap();

    let user = users::Model::create_with_password(
        &boot.app_context.db,
        &users::RegisterParams {
            email: "test@example.com".to_string(),
            password: "securepassword".to_string(),
            name: "Test User".to_string(),
        },
    )
    .await
    .unwrap();

    let pid = user.pid;

    let vars = task::Vars::from_cli_args(vec![("pid".to_string(), pid.to_string())]);

    run_task::<App>(&boot.app_context, Some(&"user:delete".to_string()), &vars)
        .await
        .unwrap();

    let user = users::Model::find_by_pid(&boot.app_context.db, &pid.to_string()).await;
    assert!(user.is_err());
    env::remove_var("TEST_CAN_RUN_USER_DELET_BY_PID");
}

