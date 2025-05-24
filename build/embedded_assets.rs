use std::collections::{HashMap, HashSet};
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub fn build_static_assets(out_dir: &Path) {
    // Determine the application root directory using Cargo environment variables
    let Some(app_dir) = find_app_directory(out_dir) else {
        eprintln!("Error: Could not determine application directory");
        return;
    };

    let app_dir_str = app_dir.to_string_lossy().to_string();

    println!("cargo:warning=Building with embedded_assets feature");
    println!("cargo:warning=Application directory: {app_dir_str}");
    println!("cargo:warning=Assets will only be loaded from the application directory");
    println!("cargo:rerun-if-changed={app_dir_str}/assets/");
    println!("cargo:rerun-if-changed={app_dir_str}/src/assets/");
    // Also run build script again if the build files change
    println!("cargo:rerun-if-changed=build/embedded_assets.rs");

    let generated_path = out_dir.join("generated_code");

    // Create the directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&generated_path) {
        eprintln!("Warning: Could not create directory: {e}");
        return;
    }

    // Only search in the application directory
    let app_root = app_dir;

    // Find all directories recursively, without filtering by name
    let all_dirs = discover_all_directories(&app_root.join("assets"));

    println!("cargo:warning=Discovered directories for assets:");
    for dir in &all_dirs {
        println!("cargo:warning=  - {}", dir.display());
    }

    // Single collection for all files
    let mut all_files = HashMap::new();

    // Store the assets directory reference to pass to collect_all_files
    let assets_dir = app_root.join("assets");

    // Process all discovered directories
    for dir in &all_dirs {
        // Process all files in this directory
        collect_all_files(dir, &assets_dir, &mut all_files);
    }

    // Generate code for all assets
    if all_files.is_empty() {
        println!("cargo:warning=No asset files found");
        // Generate empty asset files if no files found
        if let Err(e) = generate_empty_asset_files(&generated_path) {
            eprintln!("Warning: Failed to generate empty asset files: {e}");
        }
    } else {
        println!("cargo:warning=Found {} asset files", all_files.len());
        if let Err(e) = generate_asset_code(&all_files, &generated_path) {
            eprintln!("Warning: Failed to generate asset code: {e}");
        }
    }
}

pub fn find_app_directory(out_dir: &Path) -> Option<PathBuf> {
    // Find project root from OUT_DIR by going up to parent of "target" directory
    let mut path = out_dir.to_path_buf();
    while path.pop() {
        if path.file_name().and_then(|n| n.to_str()) == Some("target") && path.pop() {
            return Some(path);
        }

        // Safety check
        if path.as_os_str().is_empty() {
            break;
        }
    }

    // Fallback to current directory
    env::current_dir().ok()
}

pub fn discover_all_directories(app_root: &Path) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let mut visited = HashSet::new();

    // Only include the directory if it exists
    if app_root.exists() {
        // Add the root directory itself
        directories.push(app_root.to_path_buf());

        // Start recursive discovery
        recursively_collect_directories(app_root, &mut directories, &mut visited);
    }

    // Sort directories by their string representation to ensure consistent ordering
    directories.sort_by(|a, b| {
        a.to_string_lossy()
            .to_string()
            .cmp(&b.to_string_lossy().to_string())
    });

    directories
}

pub fn recursively_collect_directories(
    dir: &Path,
    directories: &mut Vec<PathBuf>,
    visited: &mut std::collections::HashSet<PathBuf>,
) {
    // Check if we've already visited this directory
    if !visited.insert(dir.to_path_buf()) {
        return;
    }

    // Continue recursively discovering subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Add this directory to our list
                directories.push(path.clone());
                // Continue recursion
                recursively_collect_directories(&path, directories, visited);
            }
        }
    }
}

pub fn collect_all_files(dir: &Path, assets_dir: &Path, all_files: &mut HashMap<String, String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                // Skip if we can't determine the file path or extension
                let full_path = path.to_string_lossy().to_string();

                // Create a relative path based on the assets directory
                let Ok(rel_path) = path.strip_prefix(assets_dir) else {
                    println!(
                        "cargo:warning=Failed to strip prefix for path: {}",
                        path.display()
                    );
                    continue; // Skip this file if we can't determine its relative path
                };

                // Format the key as a path, using forward slashes
                let mut key = format!("/{}", rel_path.to_string_lossy().replace('\\', "/"));

                // Remove any double slashes
                key = key.replace("//", "/");

                // Special handling for templates in views directory
                if key.starts_with("/views/") {
                    // For templates, we want to:
                    // 1. Strip "/views/" prefix for proper Tera template inheritance
                    // 2. Keep the relative path structure for nested templates
                    key = key.trim_start_matches("/views/").to_string();
                }

                // Log what we found
                println!("cargo:warning=Found asset: {} -> {}", path.display(), key);

                // Store the file
                all_files.insert(full_path, key);
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn generate_asset_code(
    all_files: &HashMap<String, String>,
    output_path: &Path,
) -> io::Result<()> {
    // Create vectors to track which files go where
    let mut static_assets = Vec::new();
    let mut template_files = Vec::new();

    // Simple categorization: if file ends with .html or .htm, it's a template, otherwise static asset
    for (path, key) in all_files {
        if std::path::Path::new(key)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("html"))
            || std::path::Path::new(key)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("htm"))
        {
            template_files.push((path.clone(), key.clone()));
        } else {
            static_assets.push((path.clone(), key.clone()));
        }
    }

    // Sort static assets by key for consistent output
    static_assets.sort_by(|a, b| a.1.cmp(&b.1));

    // Build template dependency map and sort templates
    let mut template_deps: HashMap<String, Option<String>> = HashMap::new();

    println!("cargo:warning=Analyzing template dependencies...");

    // First pass: read all template contents and find their dependencies
    for (path, key) in &template_files {
        println!("cargo:warning=Reading template: {key}");
        match fs::read_to_string(path) {
            Ok(content) => {
                // Look for {% extends "..." %} pattern
                if let Some(extends) = content
                    .lines()
                    .find(|line| line.trim().starts_with("{% extends"))
                {
                    if let Some(parent) = extends
                        .split('"')
                        .nth(1)
                        .or_else(|| extends.split('\'').nth(1))
                    {
                        template_deps.insert(key.clone(), Some(parent.to_string()));
                        println!("cargo:warning=Template {key} extends {parent}");
                    }
                } else {
                    template_deps.insert(key.clone(), None);
                    println!("cargo:warning=Template {key} has no parent");
                }
            }
            Err(e) => {
                println!("cargo:warning=Failed to read template {path}: {e}");
            }
        }
    }

    println!("cargo:warning=Template dependencies:");
    for (template, parent) in &template_deps {
        if let Some(p) = parent {
            println!("cargo:warning=  {template} -> {p}");
        } else {
            println!("cargo:warning=  {template} (no parent)");
        }
    }

    // Sort templates so that parents come before children
    let mut sorted_templates = Vec::new();
    let mut processed = HashSet::new();

    // First add all base templates (those with no parents), sorted alphabetically
    let mut base_templates: Vec<_> = template_deps
        .iter()
        .filter(|(_, parent)| parent.is_none())
        .map(|(key, _)| key.clone())
        .collect();
    base_templates.sort(); // Sort base templates alphabetically
    for key in base_templates {
        println!("cargo:warning=Adding base template: {key}");
        processed.insert(key.clone());
        sorted_templates.push(key);
    }

    // Then add all child templates, level by level
    let mut added_in_this_pass;
    while {
        added_in_this_pass = false;
        let mut level_templates = Vec::new();

        // Collect all templates at this level
        for (key, parent) in &template_deps {
            if processed.contains(key) {
                continue;
            }
            if let Some(parent) = parent {
                if processed.contains(parent) {
                    level_templates.push(key.clone());
                }
            }
        }

        // Sort templates at this level alphabetically
        level_templates.sort();

        // Add them to the final list
        for key in level_templates {
            if let Some(Some(parent)) = template_deps.get(&key) {
                println!("cargo:warning=Adding child template: {key} (extends {parent})");
            }
            processed.insert(key.clone());
            sorted_templates.push(key);
            added_in_this_pass = true;
        }

        added_in_this_pass
    } {}

    // Add any remaining templates that weren't processed, sorted alphabetically
    let mut remaining: Vec<_> = template_deps
        .keys()
        .filter(|key| !processed.contains(*key))
        .cloned()
        .collect();
    remaining.sort();
    for key in remaining {
        println!("cargo:warning=Adding unprocessed template: {key}");
        sorted_templates.push(key);
    }

    println!("cargo:warning=Final template order:");
    for (idx, template) in sorted_templates.iter().enumerate() {
        println!("cargo:warning=  {}. {}", idx + 1, template);
    }

    // Generate static assets file
    let static_file = output_path.join("static_assets.rs");

    // Create the static assets content
    let mut static_lines = vec![
        "#[must_use]\n".to_string(),
        "pub fn get_embedded_static_assets() -> std::collections::HashMap<String, &'static [u8]> {\n".to_string(),
        "    let mut assets = std::collections::HashMap::new();\n".to_string()
    ];

    for (path, key) in &static_assets {
        let insert_line = format!(
            r#"    assets.insert("{0}".to_string(), include_bytes!("{1}") as &[u8]);"#,
            key,
            path.replace('\\', "/")
        );
        static_lines.push(format!("{insert_line}\n"));
    }

    static_lines.push("    assets\n".to_string());
    static_lines.push("}\n".to_string());

    // Write static assets content to file
    let mut static_file = File::create(static_file)?;
    for line in static_lines {
        static_file.write_all(line.as_bytes())?;
    }

    // Generate templates file
    let templates_file = output_path.join("view_templates.rs");

    // Create the templates content with detailed comments
    let mut template_lines = vec![
        "/// Returns a BTreeMap of templates in dependency order (parents before children)\n"
            .to_string(),
        "#[must_use]\n".to_string(),
        "pub fn get_embedded_templates() -> std::collections::BTreeMap<String, &'static str> {\n"
            .to_string(),
        "    let mut templates = std::collections::BTreeMap::new();\n".to_string(),
    ];

    // Add templates in dependency order with comments
    for template_key in &sorted_templates {
        if let Some((path, _)) = template_files.iter().find(|(_, k)| k == template_key) {
            // Add a comment showing the dependency
            if let Some(Some(parent)) = template_deps.get(template_key) {
                template_lines.push(format!("    // Template that extends {parent}\n"));
            } else {
                template_lines.push("    // Base template with no parent\n".to_string());
            }

            let insert_line = format!(
                r#"    templates.insert("{0}".to_string(), include_str!("{1}"));"#,
                template_key,
                path.replace('\\', "/")
            );
            template_lines.push(format!("{insert_line}\n"));
        }
    }

    template_lines.push("\n    templates\n".to_string());
    template_lines.push("}\n".to_string());

    // Write templates content to file
    let mut templates_file = File::create(templates_file)?;
    for line in template_lines {
        templates_file.write_all(line.as_bytes())?;
    }

    println!(
        "cargo:warning=Generated code for {} static assets and {} templates",
        static_assets.len(),
        sorted_templates.len()
    );

    Ok(())
}

pub fn generate_empty_asset_files(output_path: &Path) -> io::Result<()> {
    // Generate empty static assets file
    let static_file = output_path.join("static_assets.rs");
    let static_code = r"#[must_use]
pub fn get_embedded_static_assets() -> std::collections::HashMap<String, &'static [u8]> {
    // No assets found
    std::collections::HashMap::new()
}
";
    let mut file = File::create(static_file)?;
    file.write_all(static_code.as_bytes())?;

    // Generate empty templates file
    let templates_file = output_path.join("view_templates.rs");
    let templates_code = r"#[must_use]
pub fn get_embedded_templates() -> std::collections::HashMap<String, &'static str> {
    // No templates found
    std::collections::HashMap::new()
}
";
    let mut file = File::create(templates_file)?;
    file.write_all(templates_code.as_bytes())?;

    Ok(())
}
