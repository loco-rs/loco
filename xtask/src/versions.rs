use std::{
    env::{self, current_dir},
    path::Path,
};

use duct::cmd;
use regex::Regex;

use crate::{
    ci::{cargo_clippy, cargo_fmt},
    errors::Result,
};

fn bump_version_in_file(
    file_path: &str,
    version_regex: &str,
    replacement_version: &str,
    once: bool,
) {
    let path = Path::new(file_path);

    // Read the content of the file
    if path.exists() {
        println!("bumping in {file_path}");
        let file_content = std::fs::read_to_string(file_path).expect("read file");

        // Apply regex replacement
        let re = Regex::new(version_regex).expect("Invalid regex");
        if !re.is_match(&file_content) {
            println!("cannot match on {file_path}");
            return;
        }
        let new_content = if once {
            re.replace(&file_content, replacement_version)
        } else {
            re.replace_all(&file_content, replacement_version)
        };

        std::fs::write(path, new_content.to_string()).expect("write file");
    }
}

pub fn bump_version(version: &str) -> Result<()> {
    // testing roco-new will test 4 combinations of starters
    // sets ROCO_DEV_MODE_PATH=/<path-to>/projects/roco/ and shared cargo build path
    let new_path = Path::new("roco-new");
    cargo_fmt(new_path)?;
    cargo_clippy(new_path)?;
    if env::var("ROCO_DEV_MODE_PATH").is_err() {
        let roco_path = current_dir()?.to_string_lossy().to_string();
        println!("setting ROCO_DEV_MODE_PATH to `{roco_path}`");
        env::set_var("ROCO_DEV_MODE_PATH", roco_path);

        // this should accelerate starters compilation
        println!("setting CARGO_SHARED_PATH");
        env::set_var("CARGO_SHARED_PATH", "/tmp/cargo-shared-path");
    }

    cmd("cargo", ["test", "--", "--test-threads", "1"].as_slice())
        .dir(new_path)
        .run()?;
    env::remove_var("CARGO_SHARED_PATH");

    // replace main versions
    let version_replacement = format!(r#"version = "{version}""#);
    bump_version_in_file("Cargo.toml", r"(?m)^version.*$", &version_replacement, true);

    bump_version_in_file(
        "roco-gen/Cargo.toml",
        r"(?m)^version.*$",
        &version_replacement,
        true,
    );

    // sync new version to subcrates in main Cargo.toml
    let roco_gen_dep = format!(r#"roco-gen = {{ version = "{version}","#);
    bump_version_in_file("Cargo.toml", r"(?m)^roco-gen [^,]*,", &roco_gen_dep, false);

    // replace the roco new version pointer
    // pub const ROCO_VERSION: &str = "0.17";
    let const_version_replacement = format!(r#"pub const ROCO_VERSION: &str = "{version}";"#);
    bump_version_in_file(
        "roco-new/src/lib.rs",
        r#"(?m)^pub const ROCO_VERSION: &str = "0.17";$"#,
        &const_version_replacement,
        true,
    );

    println!(
        "
    PUBLISHING
    
    = framework = 
    
    $ cd roco-gen && cargo publish
    $ cargo publish
    
    = roco 'new' CLI =
    
    $ cd roco-new && cargo-publish
    
    = docs =

    $ cd docs-site
    $ npm build
    $ zola build && netlify deploy -p -d public
    "
    );
    Ok(())
}
