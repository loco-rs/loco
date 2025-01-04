use std::fs;

use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;
use rstest::rstest;

use super::utils::{guess_file_by_time, MIGRATION_SRC_LIB};

#[rstest]
#[case("create_table", Component::Migration {
        name: "CreateMovies".to_string(),
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
    }, "movies.rs")]
#[case("add_column", Component::Migration {
        name: "AddNameAndAgeToUsers".to_string(),
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    }, "add_name_and_age_to_users.rs")]
#[case("remove_columns", Component::Migration {
        name: "RemoveNameAndAgeFromUsers".to_string(),
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    }, "remove_name_and_age_from_users.rs")]
#[case("add_reference", Component::Migration {
        name: "AddUserRefToPosts".to_string(),
        fields: vec![
            ("user".to_string(), "references".to_string()),
        ],
    }, "add_user_ref_to_posts.rs")]
#[case("create_join_table", Component::Migration {
        name: "CreateJoinTableUsersAndGroups".to_string(),
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    }, "create_join_table_users_and_groups.rs")]
#[case("empty", Component::Migration {
        name: "FixUsersTable".to_string(),
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
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "AddNameAndAgeToUsers".to_string(),
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "RemoveNameAndAgeFromUsers".to_string(),
        fields: vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "AddUserRefToPosts".to_string(),
        fields: vec![
            ("user".to_string(), "references".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "CreateJoinTableUsersAndGroups".to_string(),
        fields: vec![
            ("count".to_string(), "int".to_string()),
        ],
    })]
#[case(Component::Migration {
        name: "FixUsersTable".to_string(),
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
        },
    )
    .expect_err("Expected error when migration lib doesn't exist");

    assert_eq!(
        err.to_string(),
        "cannot inject into migration/src/lib.rs: file does not exist"
    );
}
