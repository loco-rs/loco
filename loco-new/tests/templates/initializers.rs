use loco::settings;
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(initializers: Option<settings::Initializers>) -> TestGenerator {
    let settings = settings::Settings {
        initializers,
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[test]
fn test_app_rs_with_initializers() {
    let generator = run_generator(Some(settings::Initializers { view_engine: true }));
    insta::assert_snapshot!(
        "src_app_rs_without_initializers",
        std::fs::read_to_string(generator.path("src/app.rs")).expect("could not open file")
    );
}

#[test]
fn test_app_rs_without_view_engine() {
    let generator = run_generator(None);
    insta::assert_snapshot!(
        "src_app_rs_with_initializers",
        std::fs::read_to_string(generator.path("src/app.rs")).expect("could not open file")
    );
}

#[rstest]
fn test_src_initializers_mod_rs_view_engine(#[values(true, false)] view_engine: bool) {
    let generator = run_generator(Some(settings::Initializers { view_engine }));

    let content = std::fs::read_to_string(generator.path("src/initializers/mod.rs"))
        .expect("could not open file");
    if view_engine {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod view_engine;$");
    } else {
        assertion::string::assert_str_not_exists(&content, "pub mod view_engine");
    }
}
