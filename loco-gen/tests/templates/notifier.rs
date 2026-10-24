use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component, RRgen};
use std::fs;

macro_rules! configure_insta {
    () => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("notifier");
        let _guard = settings.bind_to_scope();
    };
}

const LIB_RS: &str = "pub mod app;
pub mod controllers;
pub mod workers;
";

const DEV_YAML: &str = "server:
  port: 5150
";

const CARGO_TOML: &str = "[package]
name = \"tester\"

[dependencies]
loco-rs = { version = \"1.2\" }
serde = \"1\"
";

/// An app with no `src/notifiers/` yet.
fn app() -> tree_fs::Tree {
    tree_fs::TreeBuilder::default()
        .drop(true)
        .add("src/lib.rs", LIB_RS)
        .add("config/development.yaml", DEV_YAML)
        .add("config/test.yaml", DEV_YAML)
        .add("Cargo.toml", CARGO_TOML)
        .create()
        .unwrap()
}

fn run(tree: &tree_fs::Tree, name: &str) -> String {
    let rrgen = RRgen::with_working_dir(&tree.root);
    let results = generate(
        &rrgen,
        Component::Notifier {
            name: name.to_string(),
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree.root.clone(),
        },
    )
    .expect("generate notifier");
    collect_messages(&results)
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
    configure_insta!();
    let tree = app();

    let messages = run(&tree, "queue_paused");

    assert_snapshot!("messages", messages);
    assert_snapshot!(
        "generate[notifier_file]",
        read(&tree, "src/notifiers/queue_paused.rs")
    );
    assert_snapshot!(
        "inject[notifiers_mod_rs]",
        read(&tree, "src/notifiers/mod.rs")
    );
    assert_snapshot!("inject[lib_rs]", read(&tree, "src/lib.rs"));
    assert_snapshot!("edit[cargo_toml]", read(&tree, "Cargo.toml"));
    assert_snapshot!(
        "edit[development_yaml]",
        read(&tree, "config/development.yaml")
    );
    assert_snapshot!("edit[test_yaml]", read(&tree, "config/test.yaml"));
}

#[test]
fn config_stays_valid_yaml_with_an_empty_url_in_test() {
    let tree = app();
    run(&tree, "queue_paused");

    let dev = parsed(&read(&tree, "config/development.yaml"));
    assert_eq!(
        dev["notifications"]["slack"]["webhook_url"],
        serde_yaml::Value::from("env-value")
    );
    assert_eq!(dev["server"]["port"], serde_yaml::Value::from(5150));
    let test = parsed(&read(&tree, "config/test.yaml"));
    assert_eq!(
        test["notifications"]["slack"]["webhook_url"],
        serde_yaml::Value::from("")
    );
}

#[test]
fn a_second_notifier_reuses_the_module_config_and_feature() {
    let tree = app();
    run(&tree, "queue_paused");
    let config = read(&tree, "config/development.yaml");
    let cargo = read(&tree, "Cargo.toml");

    run(&tree, "HotfixLanded");

    let mod_rs = read(&tree, "src/notifiers/mod.rs");
    let mods: Vec<&str> = mod_rs
        .lines()
        .filter(|l| l.starts_with("pub mod"))
        .collect();
    assert_eq!(mods, ["pub mod queue_paused;", "pub mod hotfix_landed;"]);
    assert_eq!(
        read(&tree, "src/lib.rs")
            .matches("pub mod notifiers;")
            .count(),
        1
    );
    assert_eq!(read(&tree, "config/development.yaml"), config);
    assert_eq!(read(&tree, "Cargo.toml"), cargo);
    assert!(read(&tree, "src/notifiers/hotfix_landed.rs").contains("pub struct HotfixLanded"));
}
