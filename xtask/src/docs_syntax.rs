//! Parse every ```rust block in the docs tree and fail on the ones that are
//! not Rust.
//!
//! **This checks syntax, not types.** It will not tell you a snippet calls a
//! method that no longer exists — for that a block would have to compile
//! inside a real app, since almost every snippet names app-local types
//! (`Model`, `App`, `posts::Entity`) that do not exist in `loco-rs`. What it
//! does catch is the failure mode that actually shows up in a docs tree this
//! size: a snippet truncated mid-expression, an unbalanced brace from an
//! edit, a paste that lost its first line. Those render fine and are wrong.
//!
//! Not every block is meant to be valid Rust. Trait-signature listings have
//! no bodies; the upgrade guide compares before/after fragments; several
//! snippets elide with a literal `...`. Those opt out in the fence's meta
//! string, and must give a reason:
//!
//! ````text
//! ```rust no-syntax-check="signature listing, no body by design"
//! fn init_logger(_ctx: &AppContext) -> Result<bool>
//! ```
//! ````
//!
//! The meta is consumed by the site's code renderer, so neither the marker
//! nor the reason reaches the rendered page — verified against `dist/`. An
//! opt-out nobody has to justify is how a check stops meaning anything, so
//! the reason is not optional.

use std::{
    fs,
    path::{Path, PathBuf},
};

use eyre::{bail, eyre, Result};
use regex::Regex;

const DOCS_DIR: &str = "website/src/content/docs";
/// Crate sources whose doc comments carry ```rust blocks. `cargo test --doc`
/// compiles the runnable ones; the `ignore` ones — every example that names a
/// type from the user's app rather than from `loco-rs` — are never even
/// parsed, so this is the only thing standing between them and a truncated
/// snippet in the published API docs.
const RUST_DIRS: &[&str] = &["src", "loco-gen/src", "loco-new/src"];
const SKIP_MARKER: &str = "no-syntax-check";

struct Block {
    file: PathBuf,
    /// 1-based line of the opening fence, so the report is clickable.
    line: usize,
    code: String,
    skipped: bool,
}

pub fn run(project_dir: &Path) -> Result<()> {
    let root = project_dir.join(DOCS_DIR);
    if !root.is_dir() {
        bail!("{} does not exist", root.display());
    }

    let mut blocks = Vec::new();
    collect(&root, &mut blocks)?;
    if blocks.is_empty() {
        bail!(
            "found no ```rust blocks under {} — the extractor is broken, not the docs",
            root.display()
        );
    }

    let before_sources = blocks.len();
    for dir in RUST_DIRS {
        collect(&project_dir.join(dir), &mut blocks)?;
    }
    if blocks.len() == before_sources {
        bail!("found no ```rust blocks in {RUST_DIRS:?} — the extractor is broken, not the docs");
    }

    let mut failures = Vec::new();
    let mut checked = 0;
    let mut skipped = 0;

    for block in &blocks {
        if block.skipped {
            skipped += 1;
            continue;
        }
        checked += 1;
        if let Err(error) = parse(&block.code) {
            failures.push(format!(
                "{}:{}: not valid Rust — {error}\n    (if this is deliberate, write the fence as ```rust {SKIP_MARKER}=\"why\")",
                block.file.display(),
                block.line,
            ));
        }
    }

    println!("docs-syntax: {checked} block(s) checked, {skipped} skipped by marker");

    if failures.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "{} doc block(s) are not valid Rust:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        ))
    }
}

/// A snippet is accepted if it parses as a sequence of items (the common
/// case — a `fn`, an `impl`, a `struct`) or, failing that, as a function
/// body, which covers the bare-statement snippets.
fn parse(code: &str) -> Result<(), syn::Error> {
    syn::parse_file(code).map(|_| ()).or_else(|item_error| {
        syn::parse_str::<syn::Block>(&format!("{{\n{code}\n}}"))
            .map(|_| ())
            // The item error is the more useful of the two: nearly every
            // snippet is item-shaped, so that is the parse the author meant.
            .map_err(|_| item_error)
    })
}

/// `="signature listing"` -> `Some("signature listing")`; anything else, or an
/// empty string between the quotes, is `None`.
fn reason_at(rest: &str) -> Option<&str> {
    let quoted = rest.strip_prefix("=\"")?;
    let end = quoted.find('"')?;
    let reason = quoted[..end].trim();
    (!reason.is_empty()).then_some(reason)
}

/// Strips a Rust source down to just its doc-comment text, replacing every
/// other line with an empty one so reported line numbers still point at the
/// real file.
fn doc_comments_only(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            for marker in ["//!", "///"] {
                if let Some(rest) = trimmed.strip_prefix(marker) {
                    return rest.strip_prefix(' ').unwrap_or(rest);
                }
            }
            ""
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect(dir: &Path, out: &mut Vec<Block>) -> Result<()> {
    let fence = Regex::new(r"^\s*```\s*rust\b").expect("static regex");
    // CommonMark lets a closing fence be *longer* than the opening one, and
    // several blocks here close with four backticks.
    let closing = Regex::new(r"^\s*```+\s*$").expect("static regex");

    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<std::result::Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out)?;
            continue;
        }
        let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let contents = match extension {
            "md" | "mdx" => fs::read_to_string(&path)?,
            "rs" => doc_comments_only(&fs::read_to_string(&path)?),
            _ => continue,
        };
        let lines: Vec<&str> = contents.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            if !fence.is_match(lines[index]) {
                index += 1;
                continue;
            }
            let opened_at = index;
            // A bare `no-syntax-check` with no reason is rejected rather than
            // honoured — see the module docs.
            let skipped = match lines[opened_at].find(SKIP_MARKER) {
                None => false,
                Some(at) => {
                    let reason = reason_at(&lines[opened_at][at + SKIP_MARKER.len()..]);
                    if reason.is_none() {
                        bail!(
                            "{}:{}: `{SKIP_MARKER}` needs a reason — write it as \
                             ```rust {SKIP_MARKER}=\"why this is not Rust\"",
                            path.display(),
                            opened_at + 1
                        );
                    }
                    true
                }
            };
            index += 1;
            let start = index;
            while index < lines.len() && !closing.is_match(lines[index]) {
                index += 1;
            }
            if index >= lines.len() {
                bail!(
                    "{}:{}: ```rust block is never closed",
                    path.display(),
                    opened_at + 1
                );
            }
            out.push(Block {
                file: path.clone(),
                line: opened_at + 1,
                code: lines[start..index.min(lines.len())].join("\n"),
                skipped,
            });
            index += 1;
        }
    }
    Ok(())
}
