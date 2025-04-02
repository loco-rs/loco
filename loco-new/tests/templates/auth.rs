use loco::{settings, wizard::DBOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(enable_auth: bool, db: DBOption) -> TestGenerator {
    let settings = settings::Settings {
        package_name: "loco-app-test".to_string(),
        module_name: "loco_app_test".to_string(),
        auth: enable_auth,
        db: db.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_without_auth(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(false, DBOption::None);
    let content = assertion::yaml::load(generator.path(config_file));
    assertion::yaml::assert_path_is_empty(&content, &["auth"]);
}

#[rstest]
fn test_config_file_with_auth(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator = run_generator(true, DBOption::None);
    let content = assertion::yaml::load(generator.path(config_file));
    assertion::yaml::assert_path_key_count(&content, &["auth"], 1);

    assertion::yaml::assert_path_key_count(&content, &["auth", "jwt"], 2);
}

#[test]
fn test_config_file_development_rand_secret() {
    let generator = run_generator(true, DBOption::None);
    let content = assertion::yaml::load(generator.path("config/development.yaml"));
    assertion::yaml::assert_path_value_eq_string(
        &content,
        &["auth", "jwt", "secret"],
        "IhPi3oZCnaWvL2oIeA07",
    );
}

#[test]
fn test_config_file_test_rand_secret() {
    let generator = run_generator(true, DBOption::None);
    let content = assertion::yaml::load(generator.path("config/test.yaml"));
    assertion::yaml::assert_path_value_eq_string(
        &content,
        &["auth", "jwt", "secret"],
        "mg3ZtJzh0NoAKhdDqpQ2",
    );
}

#[rstest]
fn test_app_rs(
    #[values(true, false)] auth: bool,
    #[values(DBOption::None, DBOption::Sqlite)] db: DBOption,
) {
    let generator = run_generator(auth, db.clone());
    insta::assert_snapshot!(
        format!("src_app_rs_auth_{:?}_{:?}", auth, db),
        std::fs::read_to_string(generator.path("src/app.rs")).expect("could not open file")
    );
}

#[rstest]
fn test_src_controllers_mod_rs(#[values(true, false)] auth: bool) {
    let generator = run_generator(auth, DBOption::None);
    let content = std::fs::read_to_string(generator.path("src/controllers/mod.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod auth;$");
    } else {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod home;$");
    }
}

#[rstest]
fn test_src_views_mod_rs(#[values(true, false)] auth: bool) {
    let generator = run_generator(auth, DBOption::None);
    let content =
        std::fs::read_to_string(generator.path("src/views/mod.rs")).expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod auth;$");
    } else {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod home;$");
    }
}

#[rstest]
fn test_tests_requests_mod_rs(#[values(true, false)] auth: bool) {
    let generator = run_generator(auth, DBOption::None);
    let content = std::fs::read_to_string(generator.path("tests/requests/mod.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^mod auth;$");
        assertion::string::assert_line_regex(&content, "(?m)^mod prepare_data;$");
    } else {
        assertion::string::assert_line_regex(&content, "(?m)^mod home;$");
    }
}

#[rstest]
fn test_migration_src_lib(#[values(true)] auth: bool) {
    let generator = run_generator(auth, DBOption::Sqlite);
    let content = std::fs::read_to_string(generator.path("migration/src/lib.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^mod m20220101_000001_users;$");
        assertion::string::assert_line_regex(
            &content,
            r"(?m)Box::new\(m20220101_000001_users::Migration\),$",
        );
    }
}

#[rstest]
fn test_models_mod_rs(#[values(true)] auth: bool) {
    let generator = run_generator(auth, DBOption::Sqlite);
    let content =
        std::fs::read_to_string(generator.path("src/models/mod.rs")).expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod users;$");
    }
}

#[rstest]
fn test_models_entities_mod_rs(#[values(true)] auth: bool) {
    let generator = run_generator(auth, DBOption::Sqlite);
    let content = std::fs::read_to_string(generator.path("src/models/_entities/mod.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^pub mod users;$");
    }
}

#[rstest]
fn test_models_entities_prelude_rs(#[values(true)] auth: bool) {
    let generator = run_generator(auth, DBOption::Sqlite);
    let content = std::fs::read_to_string(generator.path("src/models/_entities/prelude.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(
            &content,
            "(?m)^pub use super::users::Entity as Users;$",
        );
    }
}

#[rstest]
fn test_tests_models_mod_rs(#[values(true, false)] auth: bool) {
    let generator = run_generator(auth, DBOption::Sqlite);
    let content = std::fs::read_to_string(generator.path("tests/models/mod.rs"))
        .expect("could not open file");

    if auth {
        assertion::string::assert_line_regex(&content, "(?m)^mod users;$");
    } else {
        assert!(content.is_empty());
    }
}
