use std::fs;

use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;

use super::utils::{guess_file_by_time, MIGRATION_SRC_LIB};

macro_rules! configure_insta {
    () => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("model");
        let _guard = settings.bind_to_scope();
    };
}

#[test]
fn can_generate() {
    std::env::set_var("SKIP_MIGRATION", "");
    configure_insta!();
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "movies".to_string(),
        link: false,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect("Generation failed");

    assert_eq!(
        collect_messages(&gen_result),
        r"* Migration for `movies` added! You can now apply it with `$ cargo loco db migrate`.
* A test for model `Movies` was added. Run with `cargo test`.
"
    );

    let migration_path = tree_fs.root.join("migration/src");
    let migration_file = guess_file_by_time(&migration_path, "m{TIME}_movies.rs", 3)
        .expect("Failed to find the generated migration file");

    assert_snapshot!(
        "generate[migration_file]",
        fs::read_to_string(&migration_file).expect("Failed to read the migration file")
    );

    with_settings!({
        filters => vec![(r"\d{8}_\d{6}", "[TIME]")]
    }, {
        assert_snapshot!(
            "inject[migration_lib]",
            fs::read_to_string(migration_path.join("lib.rs")).expect("Failed to read lib.rs")
        );
    });

    let tests_path = tree_fs.root.join("tests/models");
    assert_snapshot!(
        "generate[test_model]",
        fs::read_to_string(tests_path.join("movies.rs")).expect("Failed to read movies.rs")
    );
    assert_snapshot!(
        "inject[test_mod]",
        fs::read_to_string(tests_path.join("mod.rs")).expect("Failed to read mod.rs")
    );
}

#[test]
fn fail_when_migration_lib_not_exists() {
    std::env::set_var("SKIP_MIGRATION", "");
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "movies".to_string(),
        link: false,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let err = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect_err("Expected error when model lib doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into migration/src/lib.rs: file does not exist"
    );
}

#[test]
fn fail_when_test_models_mod_not_exists() {
    std::env::set_var("SKIP_MIGRATION", "");
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "movies".to_string(),
        link: false,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let err = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
        },
    )
    .expect_err("Expected error when migration src doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into tests/models/mod.rs: file does not exist"
    );
}
