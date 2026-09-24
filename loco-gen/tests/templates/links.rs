//! Every `loco.rs` docs link we ship inside a generated app must resolve.
//!
//! These links are read by users and by coding agents, and nothing checked
//! them: `base_template/README.md` pointed at `/docs/getting-started/tour/` and
//! `/docs/getting-started/guide/` long after the site moved to Diátaxis, so
//! every app generated in that window shipped two dead links in the one file a
//! newcomer opens first.
//!
//! Offline by construction — it resolves each URL against `website/`'s content
//! tree rather than fetching, so it costs nothing and cannot flake.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("loco-gen sits one level below the repo root")
        .to_path_buf()
}

/// Everything that ends up inside a user's app or in front of an agent.
const SHIPPED: [&str; 4] = [
    "loco-new/base_template/README.md",
    "loco-new/base_template/AGENTS.md",
    "loco-new/base_template/.claude/skills/loco",
    "loco-gen/src/templates",
];

fn markdown_and_templates(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).expect("read_dir") {
                stack.push(entry.expect("dir entry").path());
            }
        } else if path.is_file() {
            out.push(path);
        }
    }
    out
}

/// Starlight serves `website/src/content/docs/<path>.md` at `/<path>/`, so a
/// URL path maps straight onto a file (or that directory's `index.md`).
fn resolves(root: &Path, url_path: &str) -> bool {
    let base = root.join("website/src/content/docs");
    let trimmed = url_path.trim_start_matches('/').trim_end_matches('/');
    base.join(format!("{trimmed}.md")).is_file() || base.join(trimmed).join("index.md").is_file()
}

#[test]
fn every_docs_link_a_generated_app_ships_resolves_to_a_real_page() {
    let root = repo_root();
    let mut dead = Vec::new();
    let mut checked = 0usize;

    for target in SHIPPED {
        let path = root.join(target);
        if !path.exists() {
            panic!("{target} no longer exists — update this test's SHIPPED list");
        }

        for file in markdown_and_templates(&path) {
            let Ok(content) = std::fs::read_to_string(&file) else {
                continue; // binary asset (e.g. an image); nothing to scan
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
                if !resolves(&root, url) {
                    dead.push(format!(
                        "{}: https://loco.rs{url}",
                        file.strip_prefix(&root).unwrap_or(&file).display()
                    ));
                }
            }
        }
    }

    assert!(checked > 0, "scanned nothing — the link scan is broken");
    assert!(
        dead.is_empty(),
        "shipped app content links to {} page(s) that do not exist:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}
