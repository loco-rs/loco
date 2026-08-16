use super::utils::{guess_file_by_time, MIGRATION_SRC_LIB};
use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;
use rstest::rstest;
use std::fs;

#[rstest]
#[case("create_table", Component::Migration {
        name: "CreateMovies".to_string(),
        with_tz: true,
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
    }, "movies.rs")]
#[case("create_table_without_tz", Component::Migration {
        name: "CreateMovies".to_string(),
        with_tz: false,
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
    }, "movies.rs")]
#[case("add_column", Component::Migration {
        name: "AddNameAndAgeToUsers".to_string(),
        with_tz: true,
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    }, "add_name_and_age_to_users.rs")]
#[case("remove_columns", Component::Migration {
        name: "RemoveNameAndAgeFromUsers".to_string(),
        with_tz: true,
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    }, "remove_name_and_age_from_users.rs")]
#[case("add_reference", Component::Migration {
        name: "AddUserRefToPosts".to_string(),
        with_tz: true,
        fields: vec![
            ("user".to_string(), "references".to_string()),
        ],
    }, "add_user_ref_to_posts.rs")]
#[case("create_join_table_without_tz", Component::Migration {
        name: "CreateJoinTableUsersAndGroups".to_string(),
        with_tz: false,
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    }, "create_join_table_users_and_groups.rs")]
#[case("create_join_table", Component::Migration {
        name: "CreateJoinTableUsersAndGroups".to_string(),
        with_tz: true,
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    }, "create_join_table_users_and_groups.rs")]
#[case("rename_column", Component::Migration {
        name: "RenameTitleToNameOnMovies".to_string(),
        with_tz: true,
        fields: vec![],
    }, "rename_title_to_name_on_movies.rs")]
#[case("empty", Component::Migration {
        name: "FixUsersTable".to_string(),
        with_tz: true,
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    }, "fix_users_table.rs")]
#[test]
fn can_generate(
    #[case] test_name: &str,
    #[case] component: Component,
    #[case] suffix_generate_file: &str,
) {
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix(format!("{test_name}_migration"));
    let _guard = settings.bind_to_scope();

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    assert_snapshot!("generate_result", collect_messages(&gen_result));

    let migration_path = tree_fs.root.join("migration").join("src");
    let migration_file = guess_file_by_time(
        &migration_path,
        &format!("m{{TIME}}_{suffix_generate_file}"),
        3,
    )
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
}

#[rstest]
#[case(Component::Migration {
        name: "CreateMovies".to_string(),
        with_tz: true,
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "AddNameAndAgeToUsers".to_string(),
        with_tz: true,
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "RemoveNameAndAgeFromUsers".to_string(),
        with_tz: true,
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "AddUserRefToPosts".to_string(),
        with_tz: true,
        fields: vec![
            ("user".to_string(), "references".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "CreateJoinTableUsersAndGroups".to_string(),
        with_tz: true,
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "FixUsersTable".to_string(),
        with_tz: true,
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    })]
#[test]
fn fail_when_migration_lib_not_exists(#[case] component: Component) {
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("tests/models/mod.rs")
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let err = generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect_err("Expected error when migration lib doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into `migration/src/lib.rs`: file does not exist"
    );
}

/// `generate migration RenameNameOnApplicants` used to fall through to the
/// empty template, whose `up()` is `todo!()` — behind the message "Migration
/// added! You can now apply it". Following that instruction panicked. The
/// fallback is still `todo!()` (a migration that silently does nothing would
/// be worse: `db migrate` would record it as applied), but the message must no
/// longer claim the migration is runnable, and it must say which names *are*
/// understood.
#[test]
fn an_uninferrable_migration_does_not_claim_to_be_runnable() {
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    let gen_result = generate(
        &rrgen,
        Component::Migration {
            name: "FixUsersTable".to_string(),
            with_tz: true,
            fields: vec![],
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    let messages = collect_messages(&gen_result);

    assert!(
        !messages.contains("You can now apply it"),
        "an unimplemented migration must not be advertised as runnable: {messages}"
    );
    assert!(
        messages.contains("unimplemented") && messages.contains("panic"),
        "the message should say what happens if you run it anyway: {messages}"
    );
    assert!(
        messages.contains("RenameTitleToNameOnMovies"),
        "the message should list the names Loco can infer: {messages}"
    );
}

/// The inferrable forms must not regress into the `todo!()` fallback.
#[rstest]
#[case("CreateMovies", vec![("title".to_string(), "string".to_string())])]
#[case("AddNameToUsers", vec![("name".to_string(), "string".to_string())])]
#[case("RemoveNameFromUsers", vec![("name".to_string(), "string".to_string())])]
#[case("AddUserRefToPosts", vec![("user".to_string(), "references".to_string())])]
#[case("RenameTitleToNameOnMovies", vec![])]
#[case("CreateJoinTableUsersAndGroups", vec![])]
#[test]
fn an_inferrable_migration_is_never_a_todo(
    #[case] name: &str,
    #[case] fields: Vec<(String, String)>,
) {
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    generate(
        &rrgen,
        Component::Migration {
            name: name.to_string(),
            with_tz: true,
            fields,
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    let migration_src = tree_fs.root.join("migration").join("src");
    let generated = fs::read_dir(&migration_src)
        .expect("migration/src should be readable")
        .filter_map(|entry| {
            let path = entry.expect("a readable entry").path();
            (path.file_name()? != "lib.rs").then(|| fs::read_to_string(&path).unwrap_or_default())
        })
        .collect::<String>();

    assert!(
        !generated.is_empty(),
        "`{name}` should have generated a migration"
    );
    assert!(
        !generated.contains("todo!"),
        "`{name}` is an inferrable name but fell through to the empty template:\n{generated}"
    );
}
