use loco::settings;

use super::*;
use crate::assertion;

pub fn run_generator(default_features: bool, names: &[&str]) -> TestGenerator {
    let settings = settings::Settings {
        features: settings::Features {
            default_features,
            names: names.iter().map(std::string::ToString::to_string).collect(),
        },
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[test]
fn test_cargo_toml_with_default_features_and_empty_names() {
    let generator = run_generator(true, &[]);
    let content = assertion::toml::load(generator.path("Cargo.toml"));
    assertion::toml::assert_path_exists(&content, &["workspace", "dependencies", "loco-rs"]);
    assertion::toml::assert_path_is_empty(
        &content,
        &["workspace", "dependencies", "loco-rs", "default-features"],
    );
}

#[test]
fn test_cargo_toml_without_default_features_and_empty_names() {
    let generator = run_generator(false, &[]);
    let content = assertion::toml::load(generator.path("Cargo.toml"));
    assertion::toml::eq_path_value_eq_bool(
        &content,
        &["workspace", "dependencies", "loco-rs", "default-features"],
        false,
    );
}

#[test]
fn test_cargo_toml_with_features() {
    let generator = run_generator(false, &["foo", "bar"]);
    let content = assertion::toml::load(generator.path("Cargo.toml"));
    assertion::toml::assert_path_value_eq_array(
        &content,
        &["dependencies", "loco-rs", "features"],
        &[
            toml::Value::String("foo".to_string()),
            toml::Value::String("bar".to_string()),
        ],
    );
}

#[test]
fn test_cargo_toml_without_features() {
    let generator = run_generator(false, &[]);
    let content = assertion::toml::load(generator.path("Cargo.toml"));
    assertion::toml::assert_path_is_empty(&content, &["dependencies", "loco-rs", "features"]);
}
