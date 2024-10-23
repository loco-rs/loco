use std::path::Path;

use regex::Regex;

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

pub fn bump_version(version: &str) {
    for cargo in [
        "starters/saas/Cargo.toml",
        "starters/saas/migration/Cargo.toml",
    ] {
        // turn starters to local
        bump_version_in_file(
            cargo,
            // loco-rs = { version =".."
            r#"loco-rs\s*=\s*\{\s*version\s*=\s*"[^"]+""#,
            r#"loco-rs = { path="../../""#,
            false,
        );

        // turn starters from local to version
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
}
