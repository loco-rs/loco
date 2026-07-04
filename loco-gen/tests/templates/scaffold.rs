use super::utils::{guess_file_by_time, APP_ROUTS, MIGRATION_SRC_LIB, ROUTES_TSX_FIXTURE};
use insta::{assert_snapshot, with_settings};
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;
use std::fs;

/// Loco 1.0 ships a single scaffold flavor: DTO + controller + React-SPA
/// frontend, built straight from `column::Column`. The field set here is the
/// reference target from `examples/reference_spa`'s `post` resource
/// (`title:string! content:text! status:enum:draft,published,archived!
/// price:decimal! published_at:tstz`), so this snapshot doubles as the
/// golden-output check for the DTO + controller generator.
#[test]
fn can_generate() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };
    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("Api_scaffold");
    let _guard = settings.bind_to_scope();

    let component = Component::Scaffold {
        name: "post".to_string(),
        with_tz: true,
        fields: vec![
            ("title".to_string(), "string!".to_string()),
            ("content".to_string(), "text!".to_string()),
            (
                "status".to_string(),
                "enum:draft,published,archived!".to_string(),
            ),
            ("price".to_string(), "decimal!".to_string()),
            ("published_at".to_string(), "tstz".to_string()),
        ],
    };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add_empty("src/dtos/mod.rs")
        .add_empty("tests/models/mod.rs")
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add("src/app.rs", APP_ROUTS)
        .add("frontend/src/routes.tsx", ROUTES_TSX_FIXTURE)
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

    // MIGRATION
    let migration_path = tree_fs.root.join("migration/src");
    let migration_file = guess_file_by_time(&migration_path, "m{TIME}_posts.rs", 3)
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

    // MODEL TEST
    let tests_path = tree_fs.root.join("tests/models");
    assert_snapshot!(
        "generate[test_model]",
        fs::read_to_string(tests_path.join("posts.rs")).expect("Failed to read posts.rs")
    );
    assert_snapshot!(
        "inject[test_mod]",
        fs::read_to_string(tests_path.join("mod.rs")).expect("Failed to read mod.rs")
    );

    // DTO
    let dtos_path = tree_fs.root.join("src").join("dtos");
    assert_snapshot!(
        "generate[dto_file]",
        fs::read_to_string(dtos_path.join("posts.rs")).expect("dto file missing")
    );
    assert_snapshot!(
        "inject[dtos_mod_rs]",
        fs::read_to_string(dtos_path.join("mod.rs")).expect("mod.rs injection failed")
    );

    // CONTROLLER
    let controllers_path = tree_fs.root.join("src").join("controllers");
    assert_snapshot!(
        "generate[controller_file]",
        fs::read_to_string(controllers_path.join("posts.rs")).expect("controller file missing")
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

    // FRONTEND
    let frontend_path = tree_fs.root.join("frontend").join("src");
    assert_snapshot!(
        "generate[frontend_api]",
        fs::read_to_string(frontend_path.join("api").join("posts.ts"))
            .expect("frontend api file missing")
    );

    let pages_path = frontend_path.join("pages").join("posts");
    for page in ["List", "New", "Edit", "Show"] {
        assert_snapshot!(
            format!("generate[frontend_page_{page}]"),
            fs::read_to_string(pages_path.join(format!("{page}.tsx")))
                .unwrap_or_else(|_| panic!("frontend {page}.tsx file missing"))
        );
    }

    assert_snapshot!(
        "inject[routes_tsx]",
        fs::read_to_string(frontend_path.join("routes.tsx")).expect("routes.tsx injection failed")
    );

    // no `tests/requests/<plural>.rs` is generated for the API scaffold: a
    // non-compiling generated controller test is worse than none (see
    // `api/test.t`'s removal).
    assert!(!tree_fs.root.join("tests/requests").exists());
}
