//! Check that every `loco.rs/docs/` link in READMEs, shipped templates, and
//! skills resolves to a real documentation page in `website/src/content/docs`.
//!
//! Offline by construction — it resolves each URL against `website/`'s content
//! tree rather than fetching over the network, so it cannot flake.

use std::{
    fs,
    path::{Path, PathBuf},
};

use eyre::{bail, eyre, Result};

const DOCS_DIR: &str = "website/src/content/docs";

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // 1. Repo root READMEs (README.md, README.xx.md, README-xx.md)
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("README") && name_str.ends_with(".md") {
                files.push(entry.path());
            }
        }
    }

    // 2. Examples READMEs
    let examples_dir = root.join("examples");
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let readme = entry.path().join("README.md");
            if readme.is_file() {
                files.push(readme);
            }
        }
    }

    // 3. Shipped templates, agent skills, and documentation
    let specific_targets = [
        "loco-new/base_template/README.md",
        "loco-new/base_template/AGENTS.md",
        "loco-new/base_template/.claude/skills/loco",
        "loco-gen/src/templates",
        "skills/loco",
        "AGENTS.md",
    ];

    for target in specific_targets {
        let path = root.join(target);
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            walk_dir(&path, &mut files);
        }
    }

    files.sort();
    files.dedup();
    files
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Starlight serves `website/src/content/docs/<path>.md` at `/<path>/`, so a
/// URL path maps straight onto a file (or that directory's `index.md`).
fn resolves(root: &Path, url_path: &str) -> bool {
    let base = root.join(DOCS_DIR);
    let trimmed = url_path.trim_start_matches('/').trim_end_matches('/');
    base.join(format!("{trimmed}.md")).is_file() || base.join(trimmed).join("index.md").is_file()
}

/// # Errors
/// when the docs tree is missing, files cannot be read, or any docs link
/// fails to resolve to a real documentation file
pub fn run(project_dir: &Path) -> Result<()> {
    let docs_base = project_dir.join(DOCS_DIR);
    if !docs_base.is_dir() {
        bail!("{} does not exist", docs_base.display());
    }

    let files = collect_files(project_dir);
    let mut dead = Vec::new();
    let mut checked = 0usize;

    for file in files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue; // binary or non-utf8 asset
        };

        for (_, rest) in content
            .match_indices("https://loco.rs/docs/")
            .map(|(i, _)| (i, &content[i + "https://loco.rs".len()..]))
        {
            let url: String = rest
                .chars()
                .take_while(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '#')
                })
                .collect();
            // Strip a fragment and any trailing sentence punctuation the
            // link picked up from prose.
            let url = url.split('#').next().unwrap_or(&url).trim_end_matches('.');
            checked += 1;
            if !resolves(project_dir, url) {
                dead.push(format!(
                    "{}: https://loco.rs{url}",
                    file.strip_prefix(project_dir).unwrap_or(&file).display()
                ));
            }
        }
    }

    if checked == 0 {
        bail!("scanned no docs links — extractor may be broken");
    }

    println!("docs-links: {checked} link(s) checked across documentation and templates");

    if dead.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "found {} dead docs link(s):\n  {}",
            dead.len(),
            dead.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolves() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent");
        assert!(resolves(root, "/docs/tutorials/the-tour"));
        assert!(resolves(root, "/docs/tutorials/the-tour/"));
        assert!(resolves(root, "/docs/tutorials/your-first-app"));
        assert!(!resolves(root, "/docs/getting-started/tour"));
    }
}
