use sea_orm::DatabaseConnection;

use crate::{app::Hooks, Result};

/// Seeds data into the database.
///
///
/// # Errors
/// When seed fails
///
/// # Example
///
/// The provided example demonstrates how to boot the test case and run seed
/// data.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
/// use migration::Migrator;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = boot_test::<App, Migrator>().await;
///     seed::<App>(&boot.app_context.db).await.unwrap();
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn seed<H: Hooks>(db: &DatabaseConnection) -> Result<()> {
    let path = std::path::Path::new("src/fixtures");
    H::seed(db, path).await
}
