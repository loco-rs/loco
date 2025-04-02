use loco::{settings, wizard::DBOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(db: DBOption) -> TestGenerator {
    let settings = settings::Settings {
        package_name: "loco-app-test".to_string(),
        module_name: "loco_app_test".to_string(),
        db: db.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_no_db(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(DBOption::None);
    let content = assertion::yaml::load(generator.path(config_file));
    assertion::yaml::assert_path_is_empty(&content, &["database"]);
}

#[rstest]
fn test_config_with_sqlite(
    #[values(DBOption::Sqlite, DBOption::Postgres)] db: DBOption,
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(db.clone());
    let content = assertion::yaml::load(generator.path(config_file));

    insta::assert_snapshot!(
        format!(
            "{}_config_database_{:?}",
            config_file.replace(['/', '.'], "_"),
            db
        ),
        format!(
            "{:#?}",
            assertion::yaml::get_value_at_path(&content, &["database"]).unwrap()
        )
    );
}

#[rstest]
fn test_cargo_toml(#[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption) {
    let generator = run_generator(db.clone());
    let content = assertion::toml::load(generator.path("Cargo.toml"));

    insta::assert_snapshot!(
        format!("cargo_dependencies_{:?}", db),
        content.get("dependencies").unwrap()
    );
}

#[rstest]
fn test_app_rs(#[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption) {
    let generator = run_generator(db.clone());
    insta::assert_snapshot!(
        format!("src_app_rs_{:?}", db),
        std::fs::read_to_string(generator.path("src/app.rs")).expect("could not open file")
    );
}

#[rstest]
fn test_src_lib_rs(#[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption) {
    let generator = run_generator(db.clone());

    let content =
        std::fs::read_to_string(generator.path("src/lib.rs")).expect("could not open file");

    if db.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod models;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "pub mod models;");
    }
}

#[rstest]
fn test_src_bin_main_rs(
    #[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption,
) {
    let generator = run_generator(db.clone());

    let content =
        std::fs::read_to_string(generator.path("src/bin/main.rs")).expect("could not open file");

    if db.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^use migration::Migrator;$");
        assertion::string::assert_line_regex(
            &content,
            r"(?m)^    cli::main::<App, Migrator>\(\).await$",
        );
    } else {
        assertion::string::assert_str_not_exists(&content, "(?m)^use migration::Migrator;$");
        assertion::string::assert_line_regex(&content, r"(?m)^    cli::main::<App>\(\).await");
    }
}

#[rstest]
#[cfg(windows)]
fn test_src_bin_tool_rs(
    #[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption,
) {
    let generator = run_generator(db.clone());

    let content =
        std::fs::read_to_string(generator.path("src/bin/tool.rs")).expect("could not open file");

    if db.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^use migration::Migrator;$");
        assertion::string::assert_line_regex(
            &content,
            r"(?m)^    cli::main::<App, Migrator>\(\).await$",
        );
    } else {
        assertion::string::assert_str_not_exists(&content, "(?m)^use migration::Migrator;$");
        assertion::string::assert_line_regex(&content, r"(?m)^    cli::main::<App>\(\).await");
    }
}

#[rstest]
fn test_tests_mod_rs(#[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption) {
    let generator = run_generator(db.clone());

    let content =
        std::fs::read_to_string(generator.path("tests/mod.rs")).expect("could not open file");

    if db.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^mod models;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "mod models;");
    }
}
