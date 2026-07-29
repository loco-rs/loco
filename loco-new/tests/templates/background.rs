use loco::{settings, wizard::BackgroundOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(background: BackgroundOption) -> TestGenerator {
    let settings = settings::Settings {
        background: background.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_queue(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());
    let content = assertion::yaml::load(generator.path(config_file));

    match background {
        BackgroundOption::QueueRedis
        | BackgroundOption::QueuePostgres
        | BackgroundOption::QueueSqlite => {
            assertion::yaml::assert_path_is_object(&content, &["queue"]);
            assertion::yaml::assert_path_key_count(&content, &["queue"], 3);
        }
        BackgroundOption::Async | BackgroundOption::Blocking => {
            assertion::yaml::assert_path_is_empty(&content, &["queue"]);
        }
    }
}

#[rstest]
fn test_config_file_workers(
    #[values("config/development.yaml")] config_file: &str,
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());
    let content = assertion::yaml::load(generator.path(config_file));

    match background {
        BackgroundOption::Async => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "BackgroundAsync",
            );
        }
        BackgroundOption::QueueRedis
        | BackgroundOption::QueuePostgres
        | BackgroundOption::QueueSqlite => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "BackgroundQueue",
            );
        }
        BackgroundOption::Blocking => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "ForegroundBlocking",
            );
        }
    };

    assertion::yaml::assert_path_key_count(&content, &["workers"], 1);
}

#[rstest]
fn test_config_file_workers_tests(
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());
    let content = assertion::yaml::load(generator.path("config/test.yaml"));

    match background {
        BackgroundOption::Async => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "ForegroundBlocking",
            );
        }
        BackgroundOption::QueueRedis
        | BackgroundOption::QueuePostgres
        | BackgroundOption::QueueSqlite => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "ForegroundBlocking",
            );
        }
        BackgroundOption::Blocking => {
            assertion::yaml::assert_path_value_eq_string(
                &content,
                &["workers", "mode"],
                "ForegroundBlocking",
            );
        }
    };

    assertion::yaml::assert_path_key_count(&content, &["workers"], 1);
}

#[rstest]
fn test_app_rs(
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());
    insta::assert_snapshot!(
        format!("src_app_rs_{:?}", background),
        std::fs::read_to_string(generator.path("src/app.rs")).expect("could not open file")
    );
}

#[rstest]
fn test_src_lib_rs(
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());

    let content =
        std::fs::read_to_string(generator.path("src/lib.rs")).expect("could not open file");

    assertion::string::assert_line_regex(&content, "(?m)^pub mod workers;$");
}

#[rstest]
fn test_tests_mod_rs(
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres,
        BackgroundOption::QueueSqlite,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());

    let content =
        std::fs::read_to_string(generator.path("tests/mod.rs")).expect("could not open file");

    assertion::string::assert_line_regex(&content, "(?m)^mod workers;$");
}
