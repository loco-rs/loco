use std::fs;

use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("scheduler");
        let _guard = settings.bind_to_scope();
    };
}
#[test]
fn can_generate() {
    configure_insta!();
    let component = Component::Scheduler {};

    let tree_fs: tree_fs::Tree = tree_fs::TreeBuilder::default().drop(true).create().unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect("Failed to  generated scheduler file");

    assert_eq!(
        collect_messages(&gen_result),
        r"* A Scheduler job configuration was added successfully. Run with `cargo loco run scheduler --list`.
"
    );

    assert_snapshot!(
        "generate[controller_file]",
        fs::read_to_string(tree_fs.root.join("config").join("scheduler.yaml"))
            .expect("Failed to read the scheduler.yaml")
    );
}
