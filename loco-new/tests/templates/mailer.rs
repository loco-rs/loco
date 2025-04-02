use loco::settings;
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(enable_mailer: bool) -> TestGenerator {
    let settings = settings::Settings {
        mailer: enable_mailer,
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_without_mailer(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(false);
    let content = assertion::yaml::load(generator.path(config_file));
    assertion::yaml::assert_path_is_empty(&content, &["mailer"]);
}

#[rstest]
fn test_config_file_with_mailer(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(true);
    let content = assertion::yaml::load(generator.path(config_file));
    if config_file == "config/test.yaml" {
        assertion::yaml::assert_path_key_count(&content, &["mailer"], 2);
        assertion::yaml::assert_path_value_eq_bool(&content, &["mailer", "stub"], true);
    } else {
        assertion::yaml::assert_path_key_count(&content, &["mailer"], 1);
    }

    assertion::yaml::assert_path_key_count(&content, &["mailer", "smtp"], 4);
    assertion::yaml::assert_path_value_eq_bool(&content, &["mailer", "smtp", "enable"], true);
    assertion::yaml::assert_path_value_eq_int(&content, &["mailer", "smtp", "port"], 1025);
    assertion::yaml::assert_path_value_eq_bool(&content, &["mailer", "smtp", "secure"], false);
    assertion::yaml::assert_path_value_eq_string(
        &content,
        &["mailer", "smtp", "host"],
        "localhost",
    );
}

#[rstest]
fn test_cargo_toml(#[values(true, false)] mailer: bool) {
    let generator = run_generator(mailer);
    let content = assertion::toml::load(generator.path("Cargo.toml"));

    insta::assert_snapshot!(
        format!("cargo_dependencies_mailer_{:?}", mailer),
        content.get("dependencies").unwrap()
    );
}

#[rstest]
fn test_src_lib_rs(#[values(true, false)] mailer: bool) {
    let generator = run_generator(mailer);

    let content =
        std::fs::read_to_string(generator.path("src/lib.rs")).expect("could not open file");

    if mailer {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod mailers;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "pub mod mailers;;");
    }
}
