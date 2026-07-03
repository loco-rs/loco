//! `llms-check`: a drift-detection pipeline for the hand-curated LLM docs
//! files (`docs-site/static/llms.txt` and `docs-site/static/llms-full.txt`).
//!
//! This module VERIFIES the curated files against the real docs tree; it
//! never regenerates them. The curated files stay hand-authored — this just
//! catches broken links and stale version markers as the docs evolve.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::errors::{Error, Result};

const DOCS_DIR: &str = "docs-site/content/docs";
const LLMS_TXT: &str = "docs-site/static/llms.txt";
const LLMS_FULL_TXT: &str = "docs-site/static/llms-full.txt";
const AGENTS_MD: &str = "AGENTS.md";

/// The outcome of resolving a single `https://loco.rs/...` link found in
/// `llms.txt`.
enum LinkResolution {
    /// The link resolves. Carries the doc page it points at, if any (used
    /// for orphan-page detection).
    Ok(Option<PathBuf>),
    /// The link is broken, or could not be verified. Carries the reason.
    Broken(String),
}

/// Run the `llms-check` drift detector.
///
/// Checks, in order:
/// 1. every `https://loco.rs/...` link in `llms.txt` resolves to a real file
///    (ERROR on failure)
/// 2. every doc page (excluding `_index.md` and drafts) is linked from
///    `llms.txt` (WARNING only)
/// 3. the version markers in `llms.txt` and `llms-full.txt` agree on the
///    same minor version (ERROR on mismatch)
///
/// # Errors
/// when the curated files can't be read, links are broken, or the version
/// markers disagree.
pub fn run(base_dir: &Path) -> Result<()> {
    let llms_txt_path = base_dir.join(LLMS_TXT);
    let llms_full_path = base_dir.join(LLMS_FULL_TXT);

    let llms_txt = read_to_string(&llms_txt_path)?;
    let llms_full = read_to_string(&llms_full_path)?;

    let urls = extract_urls(&llms_txt);

    let mut broken_links = Vec::new();
    let mut linked_doc_pages: BTreeSet<PathBuf> = BTreeSet::new();

    for url in &urls {
        match resolve_link(base_dir, url) {
            LinkResolution::Ok(Some(doc_page)) => {
                linked_doc_pages.insert(doc_page);
            }
            LinkResolution::Ok(None) => {}
            LinkResolution::Broken(reason) => broken_links.push((url.clone(), reason)),
        }
    }

    let mut all_pages = Vec::new();
    collect_doc_pages(&base_dir.join(DOCS_DIR), &mut all_pages)?;

    let mut orphans = Vec::new();
    for page in &all_pages {
        if page.file_name().and_then(|n| n.to_str()) == Some("_index.md") {
            continue;
        }
        if is_draft(page)? {
            continue;
        }
        if !linked_doc_pages.contains(page) {
            orphans.push(page.clone());
        }
    }

    let version_mismatch = check_versions(&llms_txt, &llms_full)?;

    println!("llms-check summary:");
    println!("  links checked: {}", urls.len());
    println!("  broken links: {}", broken_links.len());
    for (url, reason) in &broken_links {
        println!("    ERROR: broken link {url} — {reason}");
    }
    println!("  orphan warnings: {}", orphans.len());
    for orphan in &orphans {
        println!(
            "    WARNING: orphan: {} not linked from llms.txt",
            display_relative(base_dir, orphan)
        );
    }
    match &version_mismatch {
        None => println!("  version markers: OK"),
        Some(msg) => println!("  version markers: MISMATCH — {msg}"),
    }

    if !broken_links.is_empty() || version_mismatch.is_some() {
        let mut msg = String::new();
        if !broken_links.is_empty() {
            msg.push_str(&format!(
                "{} broken link(s) in {LLMS_TXT}",
                broken_links.len()
            ));
        }
        if let Some(vm) = &version_mismatch {
            if !msg.is_empty() {
                msg.push_str("; ");
            }
            msg.push_str(vm);
        }
        return Err(Error::Message(format!("llms-check failed: {msg}")));
    }

    Ok(())
}

fn display_relative(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|e| Error::Message(format!("could not read {}: {e}", path.display())))
}

/// Extract every `https://loco.rs/...` URL referenced in `text`.
fn extract_urls(text: &str) -> Vec<String> {
    let re = Regex::new(r"https://loco\.rs/[^\s\)\]]+").expect("valid regex");
    let mut seen = BTreeSet::new();
    let mut urls = Vec::new();
    for m in re.find_iter(text) {
        let url = m.as_str().to_string();
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    }
    urls
}

/// Resolve a single `https://loco.rs/...` link against the repo tree.
fn resolve_link(base_dir: &Path, url: &str) -> LinkResolution {
    let Some(rest) = url.strip_prefix("https://loco.rs/") else {
        return LinkResolution::Broken("not a loco.rs URL".to_string());
    };

    // Drop any URL fragment (`#anchor`): a link still resolves to its page/file;
    // we validate page existence, not the in-page anchor.
    let rest = rest.split('#').next().unwrap_or(rest);

    if rest == "AGENTS.md" {
        return if base_dir.join(AGENTS_MD).is_file() {
            LinkResolution::Ok(None)
        } else {
            LinkResolution::Broken(format!("{AGENTS_MD} not found at repo root"))
        };
    }

    if rest == "llms-full.txt" {
        return if base_dir.join(LLMS_FULL_TXT).is_file() {
            LinkResolution::Ok(None)
        } else {
            LinkResolution::Broken(format!("{LLMS_FULL_TXT} not found"))
        };
    }

    if rest == "llms.txt" {
        return if base_dir.join(LLMS_TXT).is_file() {
            LinkResolution::Ok(None)
        } else {
            LinkResolution::Broken(format!("{LLMS_TXT} not found"))
        };
    }

    if let Some(path) = rest.strip_prefix("docs/").and_then(|p| p.strip_suffix('/')) {
        let candidates = [
            format!("{DOCS_DIR}/{path}.md"),
            format!("{DOCS_DIR}/{path}/_index.md"),
            format!("{DOCS_DIR}/{path}/index.md"),
        ];
        for candidate in &candidates {
            let candidate_path = base_dir.join(candidate);
            if candidate_path.is_file() {
                return LinkResolution::Ok(Some(candidate_path));
            }
        }
        return LinkResolution::Broken(format!(
            "no matching doc file for docs/{path}/ (checked {path}.md, {path}/_index.md, \
             {path}/index.md)"
        ));
    }

    LinkResolution::Broken("unrecognized loco.rs link, cannot verify".to_string())
}

/// Recursively collect every `*.md` file under `dir`.
fn collect_doc_pages(dir: &Path, pages: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .map_err(|e| Error::Message(format!("could not read dir {}: {e}", dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Message(format!("could not read entry in {}: {e}", dir.display()))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_doc_pages(&path, pages)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            pages.push(path);
        }
    }
    Ok(())
}

/// Whether a doc page's TOML front-matter marks it as `draft = true`.
fn is_draft(path: &Path) -> Result<bool> {
    let content = read_to_string(path)?;
    let re = Regex::new(r"(?m)^\s*draft\s*=\s*true\s*$").expect("valid regex");
    Ok(re.is_match(&content))
}

/// Extract the `X.Y` minor version from `llms.txt`'s `Current line: Loco X.Y`
/// marker and from `llms-full.txt`'s first-heading `(X.Y.x)` marker, and
/// assert they agree. Returns `Ok(Some(reason))` on mismatch, `Ok(None)` when
/// consistent.
fn check_versions(llms_txt: &str, llms_full: &str) -> Result<Option<String>> {
    let current_line_re = Regex::new(r"Current line:\s*Loco\s+(\d+\.\d+)").expect("valid regex");
    let heading_re = Regex::new(r"\((\d+\.\d+)\.x\)").expect("valid regex");

    let llms_txt_version = current_line_re
        .captures(llms_txt)
        .map(|c| c[1].to_string())
        .ok_or_else(|| {
            Error::Message(format!(
                "could not find 'Current line: Loco X.Y' marker in {LLMS_TXT}"
            ))
        })?;

    let first_heading = llms_full
        .lines()
        .find(|l| l.trim_start().starts_with('#'))
        .ok_or_else(|| {
            Error::Message(format!(
                "could not find a top-level heading in {LLMS_FULL_TXT}"
            ))
        })?;

    let llms_full_version = heading_re
        .captures(first_heading)
        .map(|c| c[1].to_string())
        .ok_or_else(|| {
            Error::Message(format!(
                "could not find version marker '(X.Y.x)' in first heading of {LLMS_FULL_TXT}"
            ))
        })?;

    if llms_txt_version == llms_full_version {
        Ok(None)
    } else {
        Ok(Some(format!(
            "version marker mismatch: {LLMS_TXT} says Loco {llms_txt_version}, {LLMS_FULL_TXT} \
             says {llms_full_version}"
        )))
    }
}
