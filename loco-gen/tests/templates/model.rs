use super::utils::{guess_file_by_time, MIGRATION_SRC_LIB};
use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;
use std::fs;

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
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };
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
        with_tz: true,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    assert_eq!(
        collect_messages(&gen_result),
        r"* Migration for `movies` added! You can now apply it with `$ cargo loco db migrate && cargo loco db entities`.
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

// Regression for #1755: a model generated with no fields must still get its
// `id` primary-key column, otherwise sea-orm produces an entity with no primary
// key that fails to compile ("Entity must have a primary key column").
#[test]
fn generate_without_fields_still_emits_primary_key() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "posts".to_string(),
        with_tz: true,
        fields: vec![],
    };

    generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    let migration_path = tree_fs.root.join("migration/src");
    let migration_file = guess_file_by_time(&migration_path, "m{TIME}_posts.rs", 3)
        .expect("Failed to find the generated migration file");
    let migration = fs::read_to_string(&migration_file).expect("read migration");

    assert!(
        migration.contains("(\"id\", ColType::PkAuto)"),
        "a field-less model must still emit its id primary key, got:\n{migration}"
    );
}

#[test]
fn fail_when_migration_lib_not_exists() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "movies".to_string(),
        with_tz: true,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let err = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect_err("Expected error when model lib doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into `migration/src/lib.rs`: file does not exist"
    );
}

#[test]
fn fail_when_test_models_mod_not_exists() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let component = Component::Model {
        name: "movies".to_string(),
        with_tz: true,
        fields: vec![("title".to_string(), "string".to_string())],
    };

    let err = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect_err("Expected error when migration src doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into `tests/models/mod.rs`: file does not exist"
    );
}

/// A migrator without the injection anchor must fail loudly.
///
/// This is the failure a user hit for real. Through rrgen 0.5, a `before:`
/// injection that could not find its anchor line rewrote the file unchanged and
/// still printed `injected: migration/src/lib.rs`. The migration file was
/// created and compiled, was never registered, therefore never ran, so the
/// table was never created and `db entities` correctly wrote nothing. Every
/// step reported success; the first insert 500s at runtime.
///
/// rrgen 0.6 turns that into an error at the point of failure and writes
/// nothing at all, so a re-run after restoring the anchor does the whole job.
/// This test guards the version floor as much as the behaviour: on an older
/// rrgen it goes green in the worst way, by generating a broken app.
#[test]
fn generating_a_model_fails_when_the_migrator_has_no_anchor() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

    // The same migrator, with the `inject-above` comment removed — exactly what
    // a hand-written or hand-edited `migration/src/lib.rs` looks like.
    let migrator_without_anchor = MIGRATION_SRC_LIB.replace(
        "            // inject-above (do not remove this comment)\n",
        "",
    );
    assert!(
        !migrator_without_anchor.contains("inject-above"),
        "fixture must have no anchor"
    );

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", &migrator_without_anchor)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let err = generate(
        &RRgen::with_working_dir(&tree_fs.root),
        Component::Model {
            name: "movies".to_string(),
            with_tz: true,
            fields: vec![("title".to_string(), "string".to_string())],
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect_err("a migration that cannot be registered must not report success");

    let message = err.to_string();
    assert!(
        message.contains("migration/src/lib.rs"),
        "the error must name the file it could not edit, got: {message}"
    );
    assert!(
        message.contains("inject-above"),
        "the error must name the anchor to restore, got: {message}"
    );
    assert!(
        message.contains("_movies::Migration"),
        "the error must show the registration that was dropped, got: {message}"
    );

    // Nothing may be left behind. A migration file written by the failed run
    // would make the retry hit `skip_glob` and return before ever reaching the
    // injection — the one state that could never repair itself.
    let migrations = std::fs::read_dir(tree_fs.root.join("migration/src"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "lib.rs")
        .collect::<Vec<_>>();
    assert!(
        migrations.is_empty(),
        "the failed generation left migrations behind: {migrations:?}"
    );
    assert_eq!(
        std::fs::read_to_string(tree_fs.root.join("migration/src/lib.rs")).unwrap(),
        migrator_without_anchor,
        "the failed generation rewrote the migrator"
    );
}

/// The check looks at the whole migration directory, so a registration that
/// went missing during an earlier run is caught on the next generation rather
/// than staying broken forever.
#[test]
fn generating_a_model_fails_when_an_earlier_migration_is_unregistered() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        // Present on disk, absent from the migrator.
        .add("migration/src/m20240101_000000_orphans.rs", "// orphan")
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let err = generate(
        &RRgen::with_working_dir(&tree_fs.root),
        Component::Model {
            name: "movies".to_string(),
            with_tz: true,
            fields: vec![("title".to_string(), "string".to_string())],
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect_err("an unregistered migration must be reported");

    assert!(
        err.to_string().contains("m20240101_000000_orphans"),
        "got: {err}"
    );
}
