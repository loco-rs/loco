{% set plural_snake = name | plural | snake_case -%}
{% set model = name | plural | pascal_case -%}
to: "tests/models/{{plural_snake}}.rs"
message: "A test for model `{{model}}` was added. Run with `cargo test`."
skip_exists: true
injections:
- into: "tests/models/mod.rs"
  append: true
  content: "mod {{plural_snake}};"
---
use {{pkg_name}}::app::App;
use migration::Migrator;
use loco_rs::testing;
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn test_model() {
    configure_insta!();

    let boot = testing::boot_test::<App, Migrator>().await.unwrap();
    testing::seed::<App>(&boot.app_context.db).await.unwrap();

    // query your model, e.g.:
    //
    // let item = models::posts::Model::find_by_pid(
    //     &boot.app_context.db,
    //     "11111111-1111-1111-1111-111111111111",
    // )
    // .await;

    // snapshot the result:
    // assert_debug_snapshot!(item);
}
