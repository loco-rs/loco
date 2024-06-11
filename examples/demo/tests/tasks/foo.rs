use blo::app::App;
use loco_rs::{boot::run_task, task, testing};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_run_foo_task() {
    let boot = testing::boot_test::<App>().await.unwrap();

    assert!(run_task::<App>(
        &boot.app_context,
        Some(&"foo".to_string()),
        &task::Vars::default()
    )
    .await
    .is_ok());
}
