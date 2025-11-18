use std::fs;

use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;

use super::utils::APP_WORKER;

macro_rules! configure_insta {
    () => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("worker");
        let _guard = settings.bind_to_scope();
    };
}

#[test]
fn can_generate() {
    configure_insta!();

    let component = Component::Worker {
        name: "register_email".to_string(),
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/workers/mod.rs")
        .add_empty("tests/workers/mod.rs")
        .add("src/app.rs", APP_WORKER)
        .create()
        .expect("Failed to create tree_fs structure");

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect("Failed to generate components");

    assert_eq!(
        collect_messages(&gen_result),
        r"* Test for worker `RegisterEmail` was added successfully. Run `cargo test`.
* A worker `RegisterEmail` was added successfully. Run with `cargo run start --worker`.
"
    );

    // Assertions for generated files
    let worker_path = tree_fs.root.join("src").join("workers");
    assert_snapshot!(
        "generate[controller_file]",
        fs::read_to_string(worker_path.join("register_email.rs"))
            .expect("Failed to read generated worker file: register_email.rs")
    );
    assert_snapshot!(
        "inject[worker_mod_rs]",
        fs::read_to_string(worker_path.join("mod.rs"))
            .expect("Failed to read updated worker mod file: mod.rs")
    );
    assert_snapshot!(
        "inject[app_rs]",
        fs::read_to_string(tree_fs.root.join("src").join("app.rs"))
            .expect("Failed to read updated app file: app.rs")
    );

    // Assertions for test files
    let tests_worker_path = tree_fs.root.join("tests").join("workers");
    assert_snapshot!(
        "generate[tests_worker_file]",
        fs::read_to_string(tests_worker_path.join("register_email.rs"))
            .expect("Failed to read generated tests worker file: register_email.rs")
    );
    assert_snapshot!(
        "inject[tests_worker_mod]",
        fs::read_to_string(tests_worker_path.join("mod.rs"))
            .expect("Failed to read updated tests worker mod file: mod.rs")
    );
}
