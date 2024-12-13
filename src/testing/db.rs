use crate::prelude::AppContext;
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
///     seed::<App>(&boot.app_context).await.unwrap();
///
///     /// .....
///     assert!(false)
/// }
/// ```
pub async fn seed<H: Hooks>(ctx: &AppContext) -> Result<()> {
    let path = std::path::Path::new("src/fixtures");
    H::seed(ctx, path).await
}
