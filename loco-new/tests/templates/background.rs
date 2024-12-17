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
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());
    let content = assertion::yaml::load(generator.path(config_file));

    if background == BackgroundOption::Queue {
        assertion::yaml::assert_path_is_object(&content, &["queue"]);
        assertion::yaml::assert_path_key_count(&content, &["queue"], 3);
        assertion::yaml::assert_path_value_eq_string(&content, &["queue", "kind"], "Redis");
        assertion::yaml::assert_path_value_eq_bool(
            &content,
            &["queue", "dangerously_flush"],
            false,
        );

        let mut inner_uri = serde_yaml::Mapping::new();
        inner_uri.insert(
            serde_yaml::Value::String("get_env(name=\"REDIS_URL\"".to_string()),
            serde_yaml::Value::Null,
        );
        inner_uri.insert(
            serde_yaml::Value::String("default=\"redis://127.0.0.1\")".to_string()),
            serde_yaml::Value::Null,
        );
        let mut uri = serde_yaml::Mapping::new();
        uri.insert(
            serde_yaml::Value::Mapping(inner_uri),
            serde_yaml::Value::Null,
        );

        assertion::yaml::assert_path_value_eq_mapping(&content, &["queue", "uri"], &uri);
    } else {
        assertion::yaml::assert_path_is_empty(&content, &["queue"]);
    }
}

#[rstest]
fn test_config_file_workers(
    #[values("config/development.yaml")] config_file: &str,
    #[values(
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
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
        BackgroundOption::Queue => {
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
        BackgroundOption::None => {
            assertion::yaml::assert_path_is_empty(&content, &["workers"]);
        }
    };

    if background.enable() {
        assertion::yaml::assert_path_key_count(&content, &["workers"], 1);
    }
}

#[rstest]
fn test_config_file_workers_tests(
    #[values(
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
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
        BackgroundOption::Queue => {
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
        BackgroundOption::None => {
            assertion::yaml::assert_path_is_empty(&content, &["workers"]);
        }
    };

    if background.enable() {
        assertion::yaml::assert_path_key_count(&content, &["workers"], 1);
    }
}

#[rstest]
fn test_app_rs(
    #[values(
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
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
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());

    let content =
        std::fs::read_to_string(generator.path("src/lib.rs")).expect("could not open file");

    if background.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod workers;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "pub mod workers;");
    }
}

#[rstest]
fn test_tests_mod_rs(
    #[values(
        BackgroundOption::None,
        BackgroundOption::Async,
        BackgroundOption::Queue,
        BackgroundOption::Blocking
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(background.clone());

    let content =
        std::fs::read_to_string(generator.path("tests/mod.rs")).expect("could not open file");

    if background.enable() {
        assertion::string::assert_line_regex(&content, "(?m)^mod workers;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "mod workers;");
    }
}
