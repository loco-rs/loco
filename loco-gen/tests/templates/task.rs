use std::fs;

use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;

use super::utils::APP_TASK;

macro_rules! configure_insta {
    () => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("task");
        let _guard = settings.bind_to_scope();
    };
}

#[test]
fn can_generate() {
    configure_insta!();

    let component = Component::Task {
        name: "cleanup".to_string(),
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/tasks/mod.rs")
        .add_empty("tests/requests/mod.rs")
        .add_empty("tests/tasks/mod.rs")
        .add("src/app.rs", APP_TASK)
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
        r"* A Task `Cleanup` was added successfully. Run with `cargo run task cleanup`.
* Tests for task `Cleanup` was added successfully. Run `cargo test`.
"
    );

    let task_path = tree_fs.root.join("src").join("tasks");
    assert_snapshot!(
        "generate[controller_file]",
        fs::read_to_string(task_path.join("cleanup.rs"))
            .expect("Failed to read generated task file: cleanup.rs")
    );
    assert_snapshot!(
        "inject[task_mod_rs]",
        fs::read_to_string(task_path.join("mod.rs"))
            .expect("Failed to read updated task mod file: mod.rs")
    );
    assert_snapshot!(
        "inject[app_rs]",
        fs::read_to_string(tree_fs.root.join("src").join("app.rs"))
            .expect("Failed to read updated app file: app.rs")
    );

    // Assertions for test files
    let tests_task_path = tree_fs.root.join("tests").join("tasks");
    assert_snapshot!(
        "generate[tests_task_file]",
        fs::read_to_string(tests_task_path.join("cleanup.rs"))
            .expect("Failed to read generated tests task file: cleanup.rs")
    );
    assert_snapshot!(
        "inject[tests_task_mod]",
        fs::read_to_string(tests_task_path.join("mod.rs"))
            .expect("Failed to read updated tests task mod file: mod.rs")
    );
}
