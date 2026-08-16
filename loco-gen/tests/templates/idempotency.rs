//! Running a generator twice must not corrupt the tree.
//!
//! Generators write files *and* inject lines into existing ones. The file write
//! is guarded by `skip_glob`, so a second run leaves the file alone — but the
//! injections are a separate mechanism with their own idea of what "already
//! there" means. If the two disagree, the second run silently appends a
//! duplicate `mod` / `Box::new(..)` line to `migration/src/lib.rs` and the app
//! stops compiling, with nothing in the output saying why.
//!
//! Nobody runs a generator twice on purpose. They run it after a typo, after a
//! `git checkout` that dropped the file, or because the first run's output
//! scrolled past.

use loco_gen::{generate, AppInfo, Component};
use rrgen::RRgen;
use rstest::rstest;
use std::fs;

use super::utils::MIGRATION_SRC_LIB;

/// Rebuilt per run because `Component` is not `Clone`.
fn build(kind: &str, name: &str, fields: &[(&str, &str)]) -> Component {
    let fields = fields
        .iter()
        .map(|(name, kind)| ((*name).to_string(), (*kind).to_string()))
        .collect();

    match kind {
        "model" => Component::Model {
            name: name.to_string(),
            with_tz: true,
            fields,
        },
        "migration" => Component::Migration {
            name: name.to_string(),
            with_tz: true,
            fields,
        },
        other => panic!("unknown component kind `{other}`"),
    }
}

#[rstest]
#[case::model("model", "movie", vec![("title", "string")])]
#[case::add_columns("migration", "AddRatingToMovies", vec![("rating", "int")])]
#[case::remove_columns("migration", "RemoveRatingFromMovies", vec![("rating", "int")])]
#[case::rename_column("migration", "RenameTitleToNameOnMovies", vec![])]
#[case::empty("migration", "FixMoviesTable", vec![])]
#[test]
fn generating_twice_does_not_duplicate_migrator_entries(
    #[case] kind: &str,
    #[case] name: &str,
    #[case] fields: Vec<(&str, &str)>,
) {
    // `generate model` shells out to `cargo loco-tool db migrate` unless told
    // not to; there is no app here to run it against.
    // SAFETY: test-local env setup; no other thread reads the environment
    // during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add_empty("tests/models/mod.rs")
        .add_empty("src/models/mod.rs")
        .create()
        .expect("should create the temp tree");

    let rrgen = RRgen::with_working_dir(&tree_fs.root);
    let appinfo = AppInfo {
        app_name: "tester".to_string(),
        working_dir: tree_fs.root.clone(),
    };

    let migrator = tree_fs.root.join("migration").join("src").join("lib.rs");

    generate(&rrgen, build(kind, name, &fields), &appinfo).expect("the first run should succeed");
    let after_first = fs::read_to_string(&migrator).expect("the migrator should be readable");

    // The second run may succeed as a no-op or refuse outright; what it must
    // not do is leave a migrator that registers the same module twice.
    let _ = generate(&rrgen, build(kind, name, &fields), &appinfo);
    let after_second = fs::read_to_string(&migrator).expect("the migrator should be readable");

    for line in after_second
        .lines()
        .filter(|line| line.contains("Box::new("))
    {
        assert_eq!(
            after_second.matches(line.trim()).count(),
            1,
            "`{}` is registered twice after generating the same component twice:\n{after_second}",
            line.trim()
        );
    }

    assert_eq!(
        after_first, after_second,
        "the second run rewrote the migrator:\n--- after one ---\n{after_first}\n--- after two \
         ---\n{after_second}"
    );
}
