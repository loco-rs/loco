use std::sync::Arc;

use loco::{
    generator::{executer::FileSystem, merge_with_default_template, Generator},
    settings,
    starter::{resolve_from, validate_setup_rhai},
    wizard::{AssetsOption, BackgroundOption, DBOption, Selections},
    OS,
};

fn default_settings() -> settings::Settings {
    let selections = Selections {
        db: DBOption::None,
        background: BackgroundOption::Async,
        asset: AssetsOption::None,
    };
    settings::Settings::from_wizard("test-app", &selections, OS::default())
}

#[test]
fn test_from_valid_template_with_setup_rhai() {
    let custom_template = tree_fs::TreeBuilder::default()
        .add_file("setup.rhai", r#"gen.copy_file(".gitignore");"#)
        .create()
        .expect("create custom template dir");

    let resolved = resolve_from(custom_template.root.to_str().unwrap(), false)
        .expect("resolve_from should succeed for a valid local path");

    validate_setup_rhai(&resolved).expect("validate_setup_rhai should succeed when setup.rhai exists");

    let merged = merge_with_default_template(&resolved)
        .expect("merge_with_default_template should succeed");

    let output_dir = tree_fs::TreeBuilder::default()
        .create()
        .expect("create output dir");

    let executor = FileSystem::new(merged.root.as_path(), output_dir.root.as_path());

    let script = std::fs::read_to_string(resolved.join("setup.rhai"))
        .expect("read setup.rhai");

    let result = Generator::new(Arc::new(executor), default_settings())
        .run_from_script(&script);

    assert!(result.is_ok(), "generator should succeed with a valid custom template");
    assert!(
        output_dir.root.join(".gitignore").exists(),
        ".gitignore should be present in the generated output"
    );
}

#[test]
fn test_from_missing_setup_rhai() {
    let custom_template = tree_fs::TreeBuilder::default()
        .add_file("some_other_file.txt", "content")
        .create()
        .expect("create custom template dir without setup.rhai");

    let resolved = resolve_from(custom_template.root.to_str().unwrap(), false)
        .expect("resolve_from should succeed: directory exists");

    let result = validate_setup_rhai(&resolved);

    assert!(result.is_err(), "validate_setup_rhai should fail when setup.rhai is missing");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(
        err.to_string().contains("setup.rhai"),
        "error message should mention 'setup.rhai'"
    );
}

#[test]
fn test_from_nonexistent_directory() {
    let result = resolve_from("/tmp/loco_test_nonexistent_from_dir_xyz987", false);

    assert!(result.is_err(), "resolve_from should fail for a nonexistent path");
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}
