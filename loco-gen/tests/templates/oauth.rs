use super::utils::APP_ROUTS;
use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component, RRgen};
use std::fs;

/// A development config with an `auth:` section already present (JWT), and
/// a section after it that must stay where it is.
const DEV_YAML: &str = "server:
  port: 5150

# Authentication Configuration
auth:
  # JWT authentication
  jwt:
    secret: abc
    expiration: 604800 # 7 days

workers:
  mode: BackgroundAsync
";

/// No `auth:` at all.
const TEST_YAML: &str = "server:
  port: 5150
";

const CARGO_TOML: &str = "[workspace]

[package]
name = \"tester\"

[workspace.dependencies]
loco-rs = { version = \"1.2\" }

[dependencies]
loco-rs = { workspace = true }
serde = \"1\"

[dev-dependencies]
loco-rs = { workspace = true, features = [\"testing\"] }
";

fn app() -> tree_fs::Tree {
    tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add("src/app.rs", APP_ROUTS)
        .add("config/development.yaml", DEV_YAML)
        .add("config/test.yaml", TEST_YAML)
        .add("config/production.yaml", TEST_YAML)
        .add("Cargo.toml", CARGO_TOML)
        .create()
        .unwrap()
}

fn run(tree: &tree_fs::Tree, provider: &str) -> loco_gen::Result<String> {
    let rrgen = RRgen::with_working_dir(&tree.root);
    let results = generate(
        &rrgen,
        Component::OAuth {
            provider: provider.to_string(),
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree.root.clone(),
        },
    )?;
    Ok(collect_messages(&results))
}

fn read(tree: &tree_fs::Tree, path: &str) -> String {
    fs::read_to_string(tree.root.join(path)).unwrap()
}

/// The config as Loco would see it, with `<%= %>` expressions stubbed out.
fn parsed(yaml: &str) -> serde_yaml::Value {
    let re = regex::Regex::new(r"<%=.*?%>").unwrap();
    serde_yaml::from_str(&re.replace_all(yaml, "env-value")).expect("config stays valid YAML")
}

#[test]
fn can_generate() {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("oauth");
    let _guard = settings.bind_to_scope();

    let tree = app();
    assert_snapshot!("generate_results", run(&tree, "github").unwrap());
    assert_snapshot!(
        "generate[controller]",
        read(&tree, "src/controllers/oauth.rs")
    );
    assert_snapshot!(
        "inject[controllers_mod]",
        read(&tree, "src/controllers/mod.rs")
    );
    assert_snapshot!("inject[app_rs]", read(&tree, "src/app.rs"));
    assert_snapshot!(
        "inject[development_yaml]",
        read(&tree, "config/development.yaml")
    );
    assert_snapshot!("inject[test_yaml]", read(&tree, "config/test.yaml"));
    assert_snapshot!(
        "inject[production_yaml]",
        read(&tree, "config/production.yaml")
    );
    assert_snapshot!("inject[cargo_toml]", read(&tree, "Cargo.toml"));
}

#[test]
fn every_environment_gets_a_parseable_provider_and_session() {
    let tree = app();
    run(&tree, "github").unwrap();
    for env in ["development", "test", "production"] {
        let config = parsed(&read(&tree, &format!("config/{env}.yaml")));
        let auth = &config["auth"];
        assert!(
            auth["oauth"]["providers"]["github"]["client_id"].is_string(),
            "{env}: {auth:?}"
        );
        assert!(auth["session"]["secret"].is_string(), "{env}");
    }
    let dev = parsed(&read(&tree, "config/development.yaml"));
    assert_eq!(
        dev["auth"]["jwt"]["secret"], "abc",
        "the JWT section survives"
    );
    assert_eq!(
        dev["workers"]["mode"], "BackgroundAsync",
        "later sections survive"
    );

    let production = read(&tree, "config/production.yaml");
    assert!(production.contains("get_env(name=\"GITHUB_CLIENT_SECRET\") %>"));
    assert!(production.contains("get_env(name=\"SESSION_SECRET\") %>"));
    assert!(
        !production.contains("change-me"),
        "production must not ship a committed secret"
    );
}

#[test]
fn a_second_provider_reuses_the_controller_and_joins_the_providers() {
    let tree = app();
    run(&tree, "github").unwrap();
    let controller = read(&tree, "src/controllers/oauth.rs");
    run(&tree, "okta").unwrap();

    assert_eq!(read(&tree, "src/controllers/oauth.rs"), controller);
    assert_eq!(
        read(&tree, "src/app.rs")
            .matches("controllers::oauth::routes()")
            .count(),
        1
    );
    assert_eq!(
        read(&tree, "src/controllers/mod.rs")
            .matches("pub mod oauth;")
            .count(),
        1
    );

    let dev = parsed(&read(&tree, "config/development.yaml"));
    let providers = &dev["auth"]["oauth"]["providers"];
    assert!(providers["github"].is_mapping());
    assert_eq!(providers["okta"]["strategy"], "generic");
    assert!(providers["okta"]["authorize_url"].is_string());
    assert_eq!(
        read(&tree, "Cargo.toml").matches("\"auth_oauth\"").count(),
        1,
        "the feature is added once"
    );
}

#[test]
fn an_existing_provider_is_an_error_and_leaves_files_alone() {
    let tree = app();
    run(&tree, "github").unwrap();
    let before = read(&tree, "config/development.yaml");
    let err = run(&tree, "github").unwrap_err().to_string();
    assert!(err.contains("already exists"), "{err}");
    assert_eq!(read(&tree, "config/development.yaml"), before);
}

#[test]
fn invalid_names_are_rejected_before_anything_is_written() {
    let tree = app();
    assert!(run(&tree, "Git Hub").is_err());
    assert!(!tree.root.join("src/controllers/oauth.rs").exists());
    assert_eq!(read(&tree, "config/test.yaml"), TEST_YAML);
}
