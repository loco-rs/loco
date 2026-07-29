use super::utils::APP_ROUTS;
use insta::assert_snapshot;
use loco_gen::{collect_messages, generate, AppInfo, Component};
use rrgen::RRgen;
use std::fs;

#[test]
fn can_generate() {
    let actions = vec!["GET".to_string(), "POST".to_string()];
    let component = Component::Controller {
        name: "movie".to_string(),
        actions: actions.clone(),
    };

    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_suffix("Api_controller");
    let _guard = settings.bind_to_scope();

    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_empty("src/controllers/mod.rs")
        .add_empty("tests/requests/mod.rs")
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

    let test_controllers_path = tree_fs.root.join("tests").join("requests");
    assert_snapshot!(
        "generate[tests_controller_mod_rs]",
        fs::read_to_string(test_controllers_path.join("movie.rs")).expect("test file missing")
    );
    assert_snapshot!(
        "inject[tests_controller_mod_rs]",
        fs::read_to_string(test_controllers_path.join("mod.rs")).expect("test mod.rs missing")
    );
}
