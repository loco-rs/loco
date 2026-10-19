use loco::{
    settings,
    wizard::{AssetsOption, BackgroundOption, DBOption, Selections},
    OS,
};
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

fn from_wizard(db: DBOption, background: BackgroundOption) -> settings::Settings {
    settings::Settings::from_wizard(
        "loco-app-test",
        &Selections {
            db,
            background,
            asset: AssetsOption::None,
        },
        OS::Linux,
    )
}

fn strings_at(content: &toml::Value, path: &[&str]) -> Vec<String> {
    assertion::toml::get_value_at_path(content, path)
        .and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// A Postgres app must not compile SQLite, through neither sea-orm's driver
/// feature nor `loco-rs`'s default `db-sqlite` feature.
#[test]
fn postgres_app_does_not_compile_sqlite() {
    let generator =
        TestGenerator::generate(from_wizard(DBOption::Postgres, BackgroundOption::Async));
    let content = assertion::toml::load(generator.path("Cargo.toml"));

    assertion::toml::eq_path_value_eq_bool(
        &content,
        &["workspace", "dependencies", "loco-rs", "default-features"],
        false,
    );
    let loco = strings_at(&content, &["dependencies", "loco-rs", "features"]);
    assert!(!loco.iter().any(|f| f == "db-sqlite"), "{loco:?}");
    assert!(loco.iter().any(|f| f == "db-postgres"), "{loco:?}");
    assert!(loco.iter().any(|f| f == "with-db"), "{loco:?}");

    let orm = strings_at(&content, &["dependencies", "sea-orm", "features"]);
    assert!(!orm.iter().any(|f| f == "sqlx-sqlite"), "{orm:?}");
    assert!(orm.iter().any(|f| f == "sqlx-postgres"), "{orm:?}");
}

/// A SQLite app must not compile Postgres, through neither sea-orm's driver
/// feature nor `loco-rs`'s default `db-postgres` feature.
#[test]
fn sqlite_app_does_not_compile_postgres() {
    let generator = TestGenerator::generate(from_wizard(DBOption::Sqlite, BackgroundOption::Async));
    let content = assertion::toml::load(generator.path("Cargo.toml"));

    assertion::toml::eq_path_value_eq_bool(
        &content,
        &["workspace", "dependencies", "loco-rs", "default-features"],
        false,
    );
    let loco = strings_at(&content, &["dependencies", "loco-rs", "features"]);
    assert!(!loco.iter().any(|f| f == "db-postgres"), "{loco:?}");
    assert!(loco.iter().any(|f| f == "db-sqlite"), "{loco:?}");

    let orm = strings_at(&content, &["dependencies", "sea-orm", "features"]);
    assert!(orm.iter().any(|f| f == "sqlx-sqlite"), "{orm:?}");
    assert!(!orm.iter().any(|f| f == "sqlx-postgres"), "{orm:?}");
}

/// A SQLite app with a Postgres queue still needs the Postgres driver.
#[test]
fn sqlite_app_with_postgres_queue_enables_the_postgres_driver() {
    let settings = from_wizard(DBOption::Sqlite, BackgroundOption::QueuePostgres);
    let names = &settings.features.names;
    assert!(names.iter().any(|n| n == "db-postgres"), "{names:?}");
}

/// `Features::defaults_without` lists the framework's default features by
/// hand, so it has to change whenever `[features] default` in loco-rs does.
#[test]
fn feature_list_matches_loco_defaults() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let content: toml::Value = toml::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
    let mut defaults = strings_at(&content, &["features", "default"]);
    defaults.sort();

    let mut ours: Vec<String> = settings::LOCO_DEFAULT_FEATURES
        .iter()
        .map(ToString::to_string)
        .collect();
    ours.sort();
    assert_eq!(ours, defaults);
}

#[test]
fn postgres_queue_does_not_list_worker_twice() {
    let settings = from_wizard(DBOption::Postgres, BackgroundOption::QueuePostgres);
    let n = settings
        .features
        .names
        .iter()
        .filter(|n| *n == "worker")
        .count();
    assert_eq!(n, 1, "{:?}", settings.features.names);
}
