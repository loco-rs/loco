use std::path::Path;

use regex::Regex;

use crate::{
    ci,
    errors::{Error, Result},
    out,
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
    let starters = [
        "starters/saas/Cargo.toml",
        "starters/rest-api/Cargo.toml",
        "starters/lightweight-service/Cargo.toml",
    ];

    // turn starters to local "../../" version for testing
    for cargo in starters {
        bump_version_in_file(
            cargo,
            // loco-rs = { version =".."
            r#"loco-rs\s*=\s*\{\s*version\s*=\s*"[^"]+""#,
            r#"loco-rs = { path="../../""#,
            false,
        );
    }

    println!("Testing starters CI");
    let starter_projects: Vec<ci::RunResults> = ci::run_all_in_folder(Path::new("starters"))?;

    println!("Starters CI results:");
    println!("{}", out::print_ci_results(&starter_projects));
    for starter in &starter_projects {
        if !starter.is_valid() {
            return Err(Error::Message(format!(
                "starter {} ins not passing the CI",
                starter.path.display()
            )));
        }
    }

    // all oK
    // turn starters from local to version
    for cargo in starters {
        bump_version_in_file(
            cargo,
            // loco-rs = { path =".."
            r#"loco-rs\s*=\s*\{\s*path\s*=\s*"[^"]+?""#,
            &format!(r#"loco-rs = {{ version = "{version}""#),
            false,
        );
    }

    // replace main versions
    let version_replacement = format!(r#"version = "{version}""#);
    bump_version_in_file("Cargo.toml", r"(?m)^version.*$", &version_replacement, true);

    bump_version_in_file(
        "loco-gen/Cargo.toml",
        r"(?m)^version.*$",
        &version_replacement,
        true,
    );

    // sync new version to subcrates in main Cargo.toml
    let loco_gen_dep = format!(r#"loco-gen = {{ version = "{version}","#);
    bump_version_in_file("Cargo.toml", r"(?m)^loco-gen [^,]*,", &loco_gen_dep, false);
    Ok(())
}
