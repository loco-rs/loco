use insta::{assert_debug_snapshot, assert_snapshot};
use std::collections::HashMap;
use std::path::Path; // For creating regex filters

// Import only the essential functions from build/embedded_assets.rs
// Use a module declaration with the `#[path]` attribute to specify the file path
#[path = "../../build/embedded_assets.rs"]
mod embedded_assets;

// Export only the functions we're actually testing
pub use embedded_assets::{
    build_static_assets, collect_all_files, discover_all_directories, find_app_directory,
    generate_asset_code, generate_empty_asset_files,
};

/// Creates a test file structure with common assets for testing.
fn create_test_assets() -> tree_fs::Tree {
    tree_fs::TreeBuilder::default()
        .drop(true)
        .add_file("assets/css/style.css", "body { color: blue; }")
        .add_file("assets/js/app.js", "console.log('Hello Loco');")
        .add_file("assets/views/index.html", "<h1>Hello World</h1>")
        .add_directory("generated")
        .create()
        .unwrap()
}

/// Creates insta settings with common filters.
fn create_insta_settings(root_path: &Path) -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(root_path.to_str().unwrap(), "[TEST_ROOT]");
    settings.add_filter("\\\\\\\\", "/");
    settings
}

#[test]
fn test_generate_empty_asset_files() {
    // Create a temporary test environment
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_directory("generated")
        .create()
        .unwrap();

    let output_path = tree_fs.root.join("generated");
    generate_empty_asset_files(&output_path).unwrap();

    // Verify files exist
    let static_file_path = output_path.join("static_assets.rs");
    let templates_file_path = output_path.join("view_templates.rs");
    assert!(
        static_file_path.exists(),
        "Static assets file should be created"
    );
    assert!(
        templates_file_path.exists(),
        "Templates file should be created"
    );

    // Use snapshots to verify file contents
    let static_file_content = std::fs::read_to_string(static_file_path).unwrap();
    let templates_file_content = std::fs::read_to_string(templates_file_path).unwrap();

    assert_snapshot!("empty_static_assets_rs", static_file_content);
    assert_snapshot!("empty_templates_rs", templates_file_content);
}

#[test]
fn test_generate_asset_code() {
    let tree_fs = create_test_assets();
    let root_path = &tree_fs.root;
    let output_path = root_path.join("generated");

    // Create file mapping
    let mut all_files = HashMap::new();
    all_files.insert(
        root_path
            .join("assets/css/style.css")
            .to_str()
            .unwrap()
            .to_string(),
        "/css/style.css".to_string(),
    );
    all_files.insert(
        root_path
            .join("assets/js/app.js")
            .to_str()
            .unwrap()
            .to_string(),
        "/js/app.js".to_string(),
    );
    all_files.insert(
        root_path
            .join("assets/views/index.html")
            .to_str()
            .unwrap()
            .to_string(),
        "index.html".to_string(),
    );

    generate_asset_code(&all_files, &output_path).unwrap();

    // Verify files exist
    let static_assets_path = output_path.join("static_assets.rs");
    let view_templates_path = output_path.join("view_templates.rs");
    assert!(static_assets_path.exists());
    assert!(view_templates_path.exists());

    // Snapshot file contents
    let static_content = std::fs::read_to_string(static_assets_path).unwrap();
    let template_content = std::fs::read_to_string(view_templates_path).unwrap();

    let settings = create_insta_settings(root_path);
    settings.bind(|| {
        assert_snapshot!("static_assets_rs", static_content);
        assert_snapshot!("view_templates_rs", template_content);
    });
}

#[test]
fn test_discover_all_directories() {
    let tree = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_directory("my_assets/css")
        .add_file("my_assets/css/style.css", "/* some css */")
        .add_directory("my_assets/js/vendor")
        .add_directory("my_assets/images")
        .add_directory("my_assets/views/user")
        .add_file("my_assets/root_file.txt", "I am a file")
        .add_directory("my_assets/empty_dir")
        .create()
        .unwrap();

    let assets_root_path = tree.root.join("my_assets");
    let discovered_dirs = discover_all_directories(&assets_root_path);

    // Use insta settings for path normalization
    let settings = create_insta_settings(&tree.root);
    settings.bind(|| {
        assert_debug_snapshot!("discovered_directories", discovered_dirs);
    });

    // Test edge cases
    let non_existent_path = tree.root.join("non_existent_assets");
    let discovered_for_non_existent = discover_all_directories(&non_existent_path);
    assert!(
        discovered_for_non_existent.is_empty(),
        "Should return empty for a non-existent path"
    );

    // Test with an empty root directory
    let empty_root_path = tree.root.join("actually_empty_assets");
    std::fs::create_dir(&empty_root_path).unwrap();
    let discovered_for_empty = discover_all_directories(&empty_root_path);
    assert_eq!(
        discovered_for_empty.len(),
        1,
        "Should find only the root for an empty existing directory"
    );
}

#[test]
fn test_find_app_directory() {
    // Case 1: Standard project structure
    let tree_target = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_file("my_project/Cargo.toml", "[package]\nname = \"my_project\"")
        .add_directory("my_project/src")
        .add_directory("my_project/target/debug/deps")
        .create()
        .unwrap();

    let expected_project_root = tree_target.root.join("my_project");
    let out_dir_in_target = expected_project_root.join("target/debug/deps");

    let app_dir = find_app_directory(&out_dir_in_target);
    assert!(
        app_dir.is_some(),
        "Should find app directory in standard structure"
    );
    if let Some(found_dir) = app_dir {
        assert_eq!(
            found_dir, expected_project_root,
            "Should find correct project root"
        );
        assert!(
            found_dir.join("Cargo.toml").exists(),
            "Located app_dir should contain Cargo.toml"
        );
    }

    // Case 2: Path not within a 'target' directory
    let tree_no_target = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_directory("some_other_place/src")
        .create()
        .unwrap();

    let path_not_in_target = tree_no_target.root.join("some_other_place/src");
    let app_dir = find_app_directory(&path_not_in_target);

    assert!(app_dir.is_some(), "Should return fallback directory");
    if let Some(found_dir) = app_dir {
        assert!(found_dir.exists(), "Fallback directory should exist");
    }
}

#[test]
fn test_build_static_assets() {
    // Create test environment
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .add_file("assets/css/style.css", "body { color: blue; }")
        .add_file("assets/js/app.js", "console.log('Hello Loco');")
        .add_file("assets/views/index.html", "<h1>Hello World</h1>")
        .add_directory("target/debug/build/embedded_code")
        .create()
        .unwrap();

    let root_path = &tree_fs.root;
    let out_dir = root_path.join("target/debug/build/embedded_code");

    // Call function being tested
    build_static_assets(&out_dir);

    // Verify files were generated
    let generated_path = out_dir.join("generated_code");
    let static_assets_path = generated_path.join("static_assets.rs");
    let view_templates_path = generated_path.join("view_templates.rs");

    assert!(generated_path.exists(), "Generated directory should exist");
    assert!(
        static_assets_path.exists(),
        "Static assets file should exist"
    );
    assert!(
        view_templates_path.exists(),
        "View templates file should exist"
    );

    // Snapshot generated files
    let static_content = std::fs::read_to_string(static_assets_path).unwrap();
    let template_content = std::fs::read_to_string(view_templates_path).unwrap();

    let settings = create_insta_settings(root_path);
    settings.bind(|| {
        assert_snapshot!("build_static_assets_static", static_content);
        assert_snapshot!("build_static_assets_templates", template_content);
    });
}

#[test]
fn test_collect_all_files() {
    let tree_fs = create_test_assets();
    let root_path = &tree_fs.root;
    let assets_dir = root_path.join("assets");

    // Test collection from css directory
    let mut all_files = HashMap::new();
    collect_all_files(&assets_dir.join("css"), &assets_dir, &mut all_files);

    // Convert to sorted vector for consistent order
    let mut file_mappings: Vec<(String, String)> = all_files
        .iter()
        .map(|(path, key)| (path.clone(), key.clone()))
        .collect();
    file_mappings.sort();

    // Use insta settings for path normalization
    let settings = create_insta_settings(root_path);
    settings.bind(|| {
        assert_debug_snapshot!("collected_css_files", file_mappings);
    });

    // Test collection from all directories
    let mut all_files = HashMap::new();
    for dir in discover_all_directories(&assets_dir) {
        collect_all_files(&dir, &assets_dir, &mut all_files);
    }

    let mut file_mappings: Vec<(String, String)> = all_files
        .iter()
        .map(|(path, key)| (path.clone(), key.clone()))
        .collect();
    file_mappings.sort();

    settings.bind(|| {
        assert_debug_snapshot!("collected_all_files", file_mappings);
    });
}

#[test]
fn test_template_inheritance() {
    // Create test environment with complex template inheritance (4 levels)
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        // Level 1 (base)
        .add_file(
            "assets/views/base.html", 
            "<!DOCTYPE html><html><head><title>{% block meta_title %}Base{% endblock %}</title>{% block head %}{% endblock %}</head><body>{% block body %}{% endblock %}</body></html>"
        )
        // Level 2 (extends base)
        .add_file(
            "assets/views/layouts/app.html", 
            "{% extends \"base.html\" %}{% block head %}<link rel=\"stylesheet\" href=\"/app.css\">{% endblock %}{% block body %}<nav>{% block nav %}{% endblock %}</nav><main>{% block content %}{% endblock %}</main>{% endblock %}"
        )
        // Level 3 (extends app)
        .add_file(
            "assets/views/layouts/authenticated.html", 
            "{% extends \"layouts/app.html\" %}{% block nav %}<div class=\"user-nav\">{% block user_nav %}{% endblock %}</div>{% endblock %}"
        )
        // Level 4 (extends authenticated)
        .add_file(
            "assets/views/dashboard/index.html", 
            "{% extends \"layouts/authenticated.html\" %}{% block meta_title %}Dashboard{% endblock %}{% block user_nav %}<a href=\"/profile\">Profile</a>{% endblock %}{% block content %}<h1>Dashboard</h1>{% endblock %}"
        )
        // Another Level 4 template to test multiple children
        .add_file(
            "assets/views/dashboard/settings.html", 
            "{% extends \"layouts/authenticated.html\" %}{% block meta_title %}Settings{% endblock %}{% block user_nav %}<a href=\"/profile\">Profile</a>{% endblock %}{% block content %}<h1>Settings</h1>{% endblock %}"
        )
        // Independent template with no inheritance
        .add_file(
            "assets/views/error.html",
            "<h1>Error</h1>"
        )
        .add_directory("target/debug/build/embedded_code")
        .create()
        .unwrap();

    let root_path = &tree_fs.root;
    let out_dir = root_path.join("target/debug/build/embedded_code");

    // Call function being tested
    build_static_assets(&out_dir);

    // Read and snapshot the generated code
    let generated_path = out_dir.join("generated_code");
    let view_templates_path = generated_path.join("view_templates.rs");
    let template_content = std::fs::read_to_string(view_templates_path).unwrap();

    let settings = create_insta_settings(root_path);
    settings.bind(|| {
        assert_snapshot!("complex_template_inheritance", template_content);
    });
}
