use demo_app::app::App;
use loco_rs::{app::AppContext, boot::run_task, task, testing};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_run_foo_task() {
    let boot = testing::boot_test::<AppContext, App>().await.unwrap();

    assert!(run_task::<AppContext, App>(
        &boot.app_context,
        Some(&"foo".to_string()),
        &task::Vars::default()
    )
    .await
    .is_ok());
}
