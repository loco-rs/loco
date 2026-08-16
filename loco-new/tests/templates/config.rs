//! Every generated config file must be usable by the app that receives it.
//!
//! These tests exist because `config/production.yaml` shipped as a **0-byte
//! file** from the CLI rewrite (#980) until 1.1: it was copied verbatim instead
//! of rendered, and no test ever opened it. Every generator test until now
//! asserted against `development.yaml` and `test.yaml` only, so the one config
//! that is hardest to try locally was the one nothing checked.
//!
//! The invariants below are therefore deliberately blunt — non-empty, parses,
//! has the sections the app's features require — and they run across the whole
//! environment set rather than the two convenient ones.

use loco::{
    settings,
    wizard::{AssetsOption, BackgroundOption, DBOption},
};
use rstest::rstest;

use super::*;
use crate::assertion;

/// Every environment the wizard writes a config for.
const CONFIG_FILES: [&str; 3] = [
    "config/development.yaml",
    "config/test.yaml",
    "config/production.yaml",
];

fn run_generator(db: DBOption, background: BackgroundOption, asset: AssetsOption) -> TestGenerator {
    let settings = settings::Settings {
        package_name: "loco-app-test".to_string(),
        module_name: "loco_app_test".to_string(),
        db: db.into(),
        background: background.into(),
        asset: asset.into(),
        auth: true,
        mailer: true,
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

/// A generated config must have content. This is the check that was missing.
#[rstest]
fn each_config_file_has_content(#[values(DBOption::None, DBOption::Sqlite)] db: DBOption) {
    let generator = run_generator(db, BackgroundOption::Async, AssetsOption::None);

    for config_file in CONFIG_FILES {
        let path = generator.path(config_file);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{config_file} was not generated: {e}"));

        assert!(
            !content.trim().is_empty(),
            "{config_file} was generated empty. An app cannot start without it, and the \
             failure only shows up in whichever environment nobody runs locally."
        );
    }
}

/// The generated `.gitignore` must not exclude the configs the app needs to run.
///
/// It used to ignore `config/production.yaml`, on the theory that production
/// config is a secret. In Loco it is not: secrets come from the environment via
/// `get_env`, and the file itself is infrastructure. Ignoring it meant the one
/// config a deploy actually needs never reached the server — you would push
/// your app, pull it on the host, and find production config missing.
///
/// `local.yaml` stays ignored. That one *is* personal.
#[test]
fn gitignore_keeps_the_configs_the_app_needs_to_run() {
    let generator = run_generator(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::None,
    );
    let gitignore =
        std::fs::read_to_string(generator.path(".gitignore")).expect("app has a .gitignore");

    for config_file in CONFIG_FILES {
        let name = config_file.trim_start_matches("config/");
        assert!(
            !gitignore.contains(name),
            ".gitignore excludes {config_file}, so it never reaches the server the app \
             deploys to:\n{gitignore}"
        );
    }

    assert!(
        gitignore.contains("config/local.yaml"),
        "local overrides should still be ignored:\n{gitignore}"
    );
}

/// A config must parse as YAML *before* it is rendered.
///
/// This is the property the YAML-safe `<%= %>` delimiters exist for
/// (<https://github.com/loco-rs/loco/issues/1727>): if a generated config is
/// not valid YAML at rest, a formatter or editor will rewrite it into
/// something that no longer starts.
#[rstest]
fn each_config_file_is_valid_yaml_unrendered(
    #[values(DBOption::None, DBOption::Sqlite, DBOption::Postgres)] db: DBOption,
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres
    )]
    background: BackgroundOption,
    #[values(AssetsOption::None, AssetsOption::Serverside, AssetsOption::Clientside)]
    asset: AssetsOption,
) {
    let generator = run_generator(db, background, asset);

    for config_file in CONFIG_FILES {
        let content = std::fs::read_to_string(generator.path(config_file))
            .unwrap_or_else(|e| panic!("{config_file} was not generated: {e}"));

        serde_yaml::from_str::<serde_yaml::Value>(&content)
            .unwrap_or_else(|e| panic!("{config_file} is not valid YAML before rendering: {e}"));
    }
}

/// The sections an app's features require must be present in every environment.
///
/// A section that exists in development but is missing from production is an
/// app that works locally and cannot boot on the server.
#[rstest]
fn feature_sections_are_present_in_every_environment(
    #[values(
        BackgroundOption::Async,
        BackgroundOption::QueueRedis,
        BackgroundOption::QueuePostgres
    )]
    background: BackgroundOption,
) {
    let generator = run_generator(DBOption::Sqlite, background, AssetsOption::None);

    for config_file in CONFIG_FILES {
        let content = assertion::yaml::load(generator.path(config_file));

        for section in ["logger", "server", "database", "auth", "workers"] {
            assert!(
                assertion::yaml::get_value_at_path(&content, &[section]).is_some(),
                "{config_file} has no `{section}` section, but the app was generated with \
                 that feature"
            );
        }
    }
}

/// Production must not inherit development's defaults for the values that
/// decide whether the app is reachable and whether its secrets are secret.
#[test]
fn production_does_not_ship_development_defaults() {
    let generator = run_generator(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::None,
    );
    let production = std::fs::read_to_string(generator.path("config/production.yaml"))
        .expect("production config was not generated");

    // Bound to loopback, the app is unreachable from outside its container.
    assert!(
        production.contains(r#"get_env(name="BINDING", default="0.0.0.0")"#),
        "production must bind 0.0.0.0, not localhost:\n{production}"
    );

    // `get_env` without a `default` fails at startup when unset — which is the
    // point: these must come from the environment, never from the repository.
    for required in [
        r#"get_env(name="DATABASE_URL")"#,
        r#"get_env(name="JWT_SECRET")"#,
        r#"get_env(name="HOST")"#,
    ] {
        assert!(
            production.contains(required),
            "production must require `{required}` rather than fall back to a development \
             value:\n{production}"
        );
    }

    // The development JWT secret is generated into the file at scaffold time.
    // If that ever lands in production, every token is forgeable by anyone who
    // can read the repository.
    let development = std::fs::read_to_string(generator.path("config/development.yaml"))
        .expect("development config was not generated");
    let development_secret = development
        .lines()
        .find_map(|line| line.trim().strip_prefix("secret: "))
        .expect("development config has a jwt secret");
    assert!(
        !production.contains(development_secret),
        "production is reusing the development JWT secret"
    );

    assert!(
        !production.contains("pretty_backtrace: true"),
        "production should not enable pretty backtraces"
    );
}
