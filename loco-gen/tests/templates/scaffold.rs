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
// A flat inventory of every artifact one scaffold emits. Splitting it into
// helpers would hide which artifact a failure came from, which is the only
// thing this test is here to tell you.
#[allow(clippy::too_many_lines)]
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
        frontend: true,
        auth: true,
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
            working_dir: tree_fs.root.clone(),
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

/// Adaptive scaffold, headless path: with `frontend: false` (a non-clientside
/// app, no `frontend/`), only the typed backend (DTO + controller) is emitted
/// and NO frontend files/dir are created.
#[test]
fn can_generate_backend_only_without_frontend() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

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
        frontend: false,
        auth: true,
    };

    // note: NO `frontend/src/routes.tsx` fixture -- this is a headless app.
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add_empty("src/dtos/mod.rs")
        .add_empty("tests/models/mod.rs")
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add("src/app.rs", APP_ROUTS)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    generate(
        &rrgen,
        component,
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    // backend emitted
    assert!(tree_fs.root.join("src/dtos/posts.rs").exists());
    assert!(tree_fs.root.join("src/controllers/posts.rs").exists());
    // frontend NOT emitted -- no orphan files or `frontend/` dir
    assert!(!tree_fs.root.join("frontend").exists());
}

/// Every resource's pages are named `List`/`New`/`Show`/`Edit`. Scaffolding a
/// *second* resource therefore injects a second set of those four names into
/// the same `routes.tsx` -- so the imports have to be aliased per resource or
/// the module has four duplicate bindings and the SPA stops building.
///
/// One scaffold could never catch this. Two can.
#[test]
fn scaffolding_a_second_resource_does_not_collide_on_import_names() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

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
    let appinfo = AppInfo {
        app_name: "tester".to_string(),
        working_dir: tree_fs.root.clone(),
    };

    for name in ["post", "movie"] {
        generate(
            &rrgen,
            Component::Scaffold {
                name: name.to_string(),
                with_tz: true,
                fields: vec![("title".to_string(), "string!".to_string())],
                frontend: true,
                auth: true,
            },
            &appinfo,
        )
        .unwrap_or_else(|e| panic!("generating the `{name}` scaffold failed: {e}"));
    }

    let routes = fs::read_to_string(tree_fs.root.join("frontend/src/routes.tsx"))
        .expect("routes.tsx should exist");

    // Both resources are wired, each under its own binding.
    for expected in [
        "import { List as PostsList } from './pages/posts/List'",
        "import { List as MoviesList } from './pages/movies/List'",
        "{ path: 'posts', element: <PostsList /> },",
        "{ path: 'movies', element: <MoviesList /> },",
    ] {
        assert!(
            routes.contains(expected),
            "routes.tsx is missing `{expected}`:\n{routes}"
        );
    }

    // And no bare binding survives -- a single `import { List }` from either
    // resource would be the collision this test exists to prevent.
    for bare in ["{ List }", "{ New }", "{ Show }", "{ Edit }"] {
        assert!(
            !routes.contains(bare),
            "routes.tsx still imports `{bare}` unaliased, which collides with the other \
             resource:\n{routes}"
        );
    }
}

/// `user:references:admin_id` names the foreign-key column explicitly. The
/// migration honours that name, so the entity has `admin_id` -- and the DTO,
/// `From<Model>`, and `Set(..)` expressions have to use it too. Deriving
/// `{target}_id` regardless produced a scaffold that referenced a column the
/// entity does not have.
#[test]
fn a_custom_foreign_key_column_name_reaches_the_dto() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add_empty("src/dtos/mod.rs")
        .add_empty("tests/models/mod.rs")
        .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
        .add("src/app.rs", APP_ROUTS)
        .create()
        .unwrap();

    let rrgen = RRgen::with_working_dir(&tree_fs.root);

    generate(
        &rrgen,
        Component::Scaffold {
            name: "movie".to_string(),
            with_tz: true,
            fields: vec![
                ("title".to_string(), "string!".to_string()),
                ("user".to_string(), "references:admin_id".to_string()),
            ],
            frontend: false,
            auth: true,
        },
        &AppInfo {
            app_name: "tester".to_string(),
            working_dir: tree_fs.root.clone(),
        },
    )
    .expect("Generation failed");

    let dto =
        fs::read_to_string(tree_fs.root.join("src/dtos/movies.rs")).expect("dto file missing");
    assert!(
        dto.contains("pub admin_id:"),
        "the DTO should carry the FK column the migration actually created:\n{dto}"
    );
    assert!(
        !dto.contains("user_id"),
        "`user_id` is the derived name the custom spec overrode; the entity has no such \
         column:\n{dto}"
    );

    let controller = fs::read_to_string(tree_fs.root.join("src/controllers/movies.rs"))
        .expect("controller file missing");
    assert!(
        !controller.contains("user_id"),
        "the controller reads/writes the entity, which has `admin_id`:\n{controller}"
    );
}

/// The scaffold is authenticated by default: every handler takes an
/// `auth::JWT` extractor, which is why a plain `curl` against a freshly
/// scaffolded resource answers 401. `--no-auth` (`auth: false`) is the
/// documented opt-out, and it has to remove the extractor from *every*
/// handler -- leaving one behind produces a resource that is half-public,
/// which is worse than either default.
#[test]
fn no_auth_generates_public_handlers() {
    // SAFETY: test-local env setup; no other thread reads the environment during this test.
    unsafe { std::env::set_var("SKIP_MIGRATION", "") };

    let generate_with_auth = |auth: bool| {
        let tree_fs = tree_fs::TreeBuilder::default()
            .drop(true)
            .add_empty("src/controllers/mod.rs")
            .add_empty("src/dtos/mod.rs")
            .add_empty("tests/models/mod.rs")
            .add("migration/src/lib.rs", MIGRATION_SRC_LIB)
            .add("src/app.rs", APP_ROUTS)
            .create()
            .unwrap();

        let rrgen = RRgen::with_working_dir(&tree_fs.root);
        generate(
            &rrgen,
            Component::Scaffold {
                name: "post".to_string(),
                with_tz: true,
                fields: vec![("title".to_string(), "string!".to_string())],
                frontend: false,
                auth,
            },
            &AppInfo {
                app_name: "tester".to_string(),
                working_dir: tree_fs.root.clone(),
            },
        )
        .expect("Generation failed");

        fs::read_to_string(tree_fs.root.join("src/controllers/posts.rs"))
            .expect("controller file missing")
    };

    // Default: all five CRUD handlers are behind the JWT extractor.
    let authenticated = generate_with_auth(true);
    assert_eq!(
        authenticated.matches("_auth: auth::JWT,").count(),
        5,
        "every scaffolded handler should be authenticated by default:\n{authenticated}"
    );

    // `--no-auth`: none of them are, and the file is otherwise unchanged.
    let public = generate_with_auth(false);
    assert!(
        !public.contains("auth::JWT"),
        "`--no-auth` should leave no JWT extractor behind:\n{public}"
    );
    assert_eq!(
        public.matches("async fn ").count(),
        5,
        "`--no-auth` should drop the extractor, not any handler:\n{public}"
    );
    // Dropping an argument must not corrupt the argument list of the handlers
    // that keep taking state.
    assert_eq!(
        public.matches("    State(ctx): State<AppContext>,").count(),
        5,
        "handler signatures should stay well-formed without the extractor:\n{public}"
    );
}
