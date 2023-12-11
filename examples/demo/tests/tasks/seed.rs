use std::collections::BTreeMap;

use blo::app::App;
use loco_rs::{boot::run_task, testing};
use migration::Migrator;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_seed_data() {
    let boot = testing::boot_test::<App, Migrator>().await.unwrap();

    let vars = BTreeMap::new();

    assert!(
        run_task::<App>(&boot.app_context, Some(&"seed_data".to_string()), &vars)
            .await
            .is_ok()
    );
}
