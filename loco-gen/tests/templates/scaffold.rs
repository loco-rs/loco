use std::fs;

use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component, ScaffoldKind};
use rrgen::RRgen;
use rstest::rstest;

use super::utils::{guess_file_by_time, APP_ROUTS, MIGRATION_SRC_LIB};

#[rstest]
#[case(ScaffoldKind::Api)]
#[case(ScaffoldKind::Html)]
#[case(ScaffoldKind::Htmx)]
#[test]
fn can_generate(#[case] kind: ScaffoldKind) {
    std::env::set_var("SKIP_MIGRATION", "");
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix(format!("{kind:?}_scaffold"));
    let _guard = settings.bind_to_scope();

    let component = Component::Scaffold {
        name: "movie".to_string(),
        fields: vec![
            ("title".to_string(), "string".to_string()),
            ("user".to_string(), "references".to_string()),
        ],
        kind: kind.clone(),
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add_empty("tests/models/mod.rs")
        .add_empty("src/views/mod.rs")
        .add_empty("tests/requests/mod.rs")
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add("src/app.rs", APP_ROUTS)
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

    assert_snapshot!("generate_results", collect_messages(&gen_result));

    // MODELS
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
    with_settings!({
        filters => vec![(r"\d{8}_\d{6}", "[TIME]")]
    }, {
        assert_snapshot!(
            "inject[migration_lib]",
            fs::read_to_string(migration_path.join("lib.rs")).expect("Failed to read lib.rs")
        );
    });

    // CONTROLLER
    let controllers_path = tree_fs.root.join("src").join("controllers");
    assert_snapshot!(
        "generate[controller_file]",
        fs::read_to_string(controllers_path.join("movie.rs")).expect("controller file missing")
    );

    assert_snapshot!(
        "inject[controller_mod_rs]",
        fs::read_to_string(controllers_path.join("mod.rs")).expect("mod.rs injection failed")
    );

    assert_snapshot!(
        "inject[app_rs]",
        fs::read_to_string(tree_fs.root.join("src").join("app.rs"))
            .expect("app.rs injection failed")
    );

    // TESTS
    let tests_path = tree_fs.root.join("tests/models");
    assert_snapshot!(
        "generate[test_model]",
        fs::read_to_string(tests_path.join("movies.rs")).expect("Failed to read movies.rs")
    );
    assert_snapshot!(
        "inject[test_mod]",
        fs::read_to_string(tests_path.join("mod.rs")).expect("Failed to read mod.rs")
    );

    // VIEWS
    match kind {
        ScaffoldKind::Api => (),
        ScaffoldKind::Html | ScaffoldKind::Htmx => {
            let base_views_path = tree_fs.root.join("src").join("views");
            assert_snapshot!(
                "generate[views_rs]",
                fs::read_to_string(base_views_path.join("movie.rs"))
                    .expect("Failed to read mod.rs")
            );
            assert_snapshot!(
                "inject[views_mod_rs]",
                fs::read_to_string(base_views_path.join("mod.rs")).expect("Failed to read mod.rs")
            );

            let views_path = tree_fs.root.join("assets").join("views").join("movie");
            let views = vec!["create", "edit", "list", "show"];
            for view in views {
                assert_snapshot!(
                    format!("generate[views_[{view}]]"),
                    fs::read_to_string(views_path.join(format!("{view}.html")))
                        .expect("view file missing")
                );
            }
        }
    }
}

// thread 'templates::scaffold::can_generate::case_1' panicked at
// loco-gen/tests/templates/scaffold.rs:48:6:
