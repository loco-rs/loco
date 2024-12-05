use loco::{settings, wizard::DBOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator() -> TestGenerator {
    let settings = settings::Settings {
        package_name: "loco-app-test".to_string(),
        module_name: "loco_app_test".to_string(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[test]
fn test_cargo_toml() {
    let generator = run_generator();

    let content = assertion::toml::load(generator.path("Cargo.toml"));

    assertion::toml::assert_path_value_eq_string(&content, &["package", "name"], "loco-app-test");
    assertion::toml::assert_path_value_eq_string(
        &content,
        &["package", "default-run"],
        "loco_app_test-cli",
    );

    let bin = content
        .get("bin")
        .expect("bin")
        .get(0)
        .expect("get first bin");
    assertion::toml::assert_path_value_eq_string(bin, &["name"], "loco_app_test-cli");
}

#[rstest]
fn test_use_name(#[values("src/bin/main.rs", "tests/requests/home.rs")] file: &str) {
    let generator = run_generator();

    let content = std::fs::read_to_string(generator.path(file)).expect("could not open file");

    assertion::string::assert_line_regex(&content, "(?m)^use loco_app_test::");
}

#[rstest]
fn test_use_name_with_db(
    #[values("tests/models/users.rs", "tests/requests/prepare_data.rs")] file: &str,
) {
    let generator = super::db::run_generator(DBOption::Sqlite);

    let content = std::fs::read_to_string(generator.path(file)).expect("could not open file");

    assertion::string::assert_line_regex(&content, "(?m)^use loco_app_test::");
}

#[rstest]
fn test_use_name_with_auth(#[values("tests/requests/auth.rs")] file: &str) {
    let generator = super::auth::run_generator(true);

    let content = std::fs::read_to_string(generator.path(file)).expect("could not open file");

    assertion::string::assert_line_regex(&content, "(?m)^use loco_app_test::");
}
