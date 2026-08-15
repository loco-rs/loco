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

/// The JWT `location` setting (cookie / query parameter) was undiscoverable:
/// its working YAML is `location: {from: Cookie, name: ..}`, and every wrong
/// shape collapses to `data did not match any variant of untagged enum
/// JWTLocationConfig`. The generated config is where a developer looks first,
/// so the example lives there — and a stray Tera whitespace trim would
/// silently swallow it, which is what this asserts against.
#[test]
fn the_generated_config_shows_how_to_move_the_jwt_out_of_the_header() {
    let generator = run_generator(true, DBOption::None);
    let raw = std::fs::read_to_string(generator.path("config/development.yaml"))
        .expect("development.yaml should exist");

    assert!(
        raw.contains("# location:") && raw.contains("#   from: Cookie"),
        "the commented `location` example should survive rendering:\n{raw}"
    );
}

/// A snapshot of a whole `users::Model` encodes every column, so adding one
/// field to `users` — the first thing most apps do — invalidates every such
/// test at once. Five of them did, and the one real change drowned in the
/// mechanical re-blessing. The template's tests snapshot narrow projections
/// now; this keeps them that way.
#[test]
fn no_generated_test_snapshots_a_whole_model() {
    let generator = run_generator(true, DBOption::Sqlite);

    let mut offenders = Vec::new();
    for dir in ["tests/models/snapshots", "tests/requests/snapshots"] {
        let path = generator.path(dir);
        let entries = std::fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("{} should exist: {e}", path.display()));

        for entry in entries {
            let entry = entry.expect("a readable directory entry");
            let contents = std::fs::read_to_string(entry.path()).expect("a readable snapshot");
            if contents
                .lines()
                .any(|line| line.trim_start().starts_with("Model {"))
            {
                offenders.push(entry.path());
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these snapshots pin a whole model, so any added column breaks them; \
         snapshot the fields under test instead: {offenders:#?}"
    );
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
