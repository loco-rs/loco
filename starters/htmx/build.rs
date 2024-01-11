// WARNING: Magic ahead. Don't modify unless you know what you're doing.
// This build script is used to replace the dependency line in Cargo.toml
// when developing the loco framework itself. Otherwise, the crate will
// be pulled from crates.io, and will not reflect changes we are making while
// developing the framework.

use std::env;
use std::fs;
use std::path::Path;
// use toml;

fn main() {
    // Bind the environment variable to a variable
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir_path = Path::new(&manifest_dir);

    // Now you can use manifest_dir_path to navigate to the grandparent directory
    let grandparent_path = manifest_dir_path.parent().unwrap().parent().unwrap();

    // Check if the grandparent directory is 'loco'
    if grandparent_path.file_name().unwrap() == "loco" {
        let cargo_toml_path = manifest_dir_path.join("Cargo.toml");
        let mut cargo_toml_contents = fs::read_to_string(&cargo_toml_path)
            .expect("Failed to read Cargo.toml");

        // let version = get_version_from_cargo_toml().expect("Failed to get version from parent project's Cargo.toml");
        let version = "0.1.9";
        let replacement_str = format!("loco-rs = {{ version = \"{}\"", version);

        // Replace the dependency line
        cargo_toml_contents = cargo_toml_contents.replace(
            &replacement_str,
            "loco-rs = { path = \"../../\"",
        );

        // Write back to Cargo.toml
        fs::write(&cargo_toml_path, cargo_toml_contents)
            .expect("Failed to write to Cargo.toml");
    }
}

// fn get_version_from_cargo_toml() -> Result<String, Box<dyn std::error::Error>> {
//     let path = Path::new("../../Cargo.toml");

//     let contents = fs::read_to_string(path)?;
//     let parsed = toml::from_str::<toml::Value>(&contents)?;

//     let version = parsed
//         .get("package")
//         .and_then(|pkg| pkg.get("version"))
//         .and_then(|v| v.as_str())
//         .ok_or("Version not found")?
//         .to_string();

//     Ok(version)
// }