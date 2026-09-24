//! Keeps the `loco` agent skill correct and single-sourced.
//!
//! # Why this exists
//!
//! Coding agents are not trained on Loco. Left to themselves they either invent
//! plausible-but-nonexistent APIs (`loco_rs::app::AppState`, `loco_rs::db::Pool`)
//! or burn dozens of turns grepping ~36k lines of framework source to answer
//! "does this symbol exist and what is it called?".
//!
//! `skills/loco/` answers both, and this command owns the two parts of it that
//! a human cannot be trusted to maintain:
//!
//! 1. **`api-index.md`** — the crate's public surface, derived from rustdoc
//!    JSON. A hand-written API list would rot within one release.
//! 2. **The copy shipped into generated apps.** The skill has to reach app
//!    authors, who do not have this repo — so `loco new` writes it into every
//!    app. Maintaining that copy by hand would recreate the exact divergence
//!    this command exists to prevent (the two `AGENTS.md` files already did).
//!
//! `--check` fails CI when either has drifted.
//!
//! # Requirements
//!
//! rustdoc JSON is nightly-only (`-Zunstable-options`), and the toolchain must
//! be at or above the workspace MSRV or the crate will not build under it.
//! Both are verified up front — a silently-skipped regeneration would let the
//! index rot invisibly, which is the exact failure this module exists to
//! prevent.

use std::{
    // Writing into a `String` is infallible — the `Result` exists only to
    // satisfy the trait — so the `write!` calls in `emit` discard it.
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

/// The canonical skill directory, relative to the project root.
const SKILL_DIR: &str = "skills/loco";

/// The crates an agent needs in order to write a Loco app, and where each
/// one's index is written.
///
/// A Loco app is Loco *plus Sea-ORM* — the ORM is not an implementation detail
/// the framework hides, it is API the author writes directly. Indexing only
/// `loco_rs` left the larger half undocumented, and eval submissions failed on
/// Sea-ORM query builders (`Expr::like`) that no amount of Loco documentation
/// could have prevented. Axum is deliberately absent: models already know it
/// well, and every file added here is context spent.
///
/// `(package, crate file stem, output path, workspace member)`
const CRATES: &[(&str, &str, &str, bool)] = &[
    ("loco-rs", "loco_rs", "skills/loco/api-index.md", true),
    ("sea-orm", "sea_orm", "skills/loco/sea-orm-index.md", false),
];

/// Where `loco new` picks the skill up from, relative to the project root.
///
/// `.claude/skills/` is the Agent Skills convention; the generated app's
/// `AGENTS.md` points at it so non-Claude tools find it too.
const SHIPPED_DIR: &str = "loco-new/base_template/.claude/skills/loco";

/// Items whose paths start with these segments are implementation surface that
/// an app author never names directly.
const SKIP_PREFIXES: &[&str] = &["tests", "testing::prelude::__"];

/// Regenerate the API index and mirror the skill into the app template.
///
/// # Errors
///
/// Returns an error if the nightly toolchain is missing or below MSRV, if
/// rustdoc fails, or — in `check` mode — if either the index or the shipped
/// copy is stale.
pub fn run(project_dir: &Path, check: bool) -> eyre::Result<()> {
    for (package, stem, out, workspace) in CRATES {
        let json = rustdoc_json(project_dir, package, stem, *workspace)?;
        let doc: Value = serde_json::from_str(&json)?;
        let rendered = render(&doc, package)?;

        let target = project_dir.join(out);
        if check {
            let existing = fs::read_to_string(&target).unwrap_or_default();
            if existing.trim() != rendered.trim() {
                eyre::bail!(
                    "{out} is stale — {package}'s public API changed. \
                     Regenerate with `cargo xtask agent-skill` and commit the result."
                );
            }
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, &rendered)?;
            println!(
                "agent-skill: wrote {} ({} bytes)",
                target.display(),
                rendered.len()
            );
        }
    }

    mirror(project_dir, check)?;
    root_agents_md(project_dir, check)?;
    Ok(())
}

/// Regenerate the repo-root `AGENTS.md` from the skill's `SKILL.md`.
///
/// The root guide and the skill router answer the same question for the same
/// reader, and keeping them as two hand-written files is what let the root one
/// drift — it still claimed to target Loco 1.0 several releases later.
/// Deriving it removes the possibility.
fn root_agents_md(project_dir: &Path, check: bool) -> eyre::Result<()> {
    let skill = fs::read_to_string(project_dir.join(SKILL_DIR).join("SKILL.md"))?;

    // Strip the YAML frontmatter: it configures skill loading and means
    // nothing to someone reading AGENTS.md directly.
    let body = skill
        .strip_prefix("---")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(skill.as_str(), |(_, body)| body)
        .trim_start();

    // Absolute URLs, because this file is served at <https://loco.rs/AGENTS.md>
    // as well as read in-repo, and a repo-relative link is a dead end there.
    let rendered = format!(
        "<!-- GENERATED by `cargo xtask agent-skill` from skills/loco/SKILL.md. \
         Do not edit by hand. -->\n\n{body}\n\
         ---\n\n\
         ## Where these files are\n\n\
         The files referenced above are published at \
         <https://loco.rs/skills/loco/> — for example\n\
         <https://loco.rs/skills/loco/doctrine.md> and \
         <https://loco.rs/skills/loco/api-index.md>.\n\n\
         They also live in `skills/loco/` in the \
         [loco repository](https://github.com/loco-rs/loco), and `loco new` writes\n\
         them into every generated app under `.claude/skills/loco/`, matched to that\n\
         app's `loco-rs` version. If you are working inside a Loco app, prefer that\n\
         local copy — it cannot be out of step with the crate the app compiles\n\
         against.\n"
    );

    let target = project_dir.join("AGENTS.md");
    if check {
        if fs::read_to_string(&target).unwrap_or_default().trim() != rendered.trim() {
            eyre::bail!("AGENTS.md is stale. Run `cargo xtask agent-skill` and commit the result.");
        }
        return Ok(());
    }
    fs::write(&target, &rendered)?;
    println!("agent-skill: wrote AGENTS.md from SKILL.md");
    Ok(())
}

/// Mirror `skills/loco/` into the app template so `loco new` ships it.
fn mirror(project_dir: &Path, check: bool) -> eyre::Result<()> {
    let src = project_dir.join(SKILL_DIR);
    let dst = project_dir.join(SHIPPED_DIR);

    let mut files = Vec::new();
    collect(&src, &src, &mut files)?;
    files.sort();

    if check {
        let mut shipped = Vec::new();
        if dst.exists() {
            collect(&dst, &dst, &mut shipped)?;
        }
        shipped.sort();
        if shipped != files {
            eyre::bail!(
                "{SHIPPED_DIR} does not match {SKILL_DIR} (file set differs). \
                 Run `cargo xtask agent-skill` and commit the result."
            );
        }
        for rel in &files {
            if fs::read(src.join(rel))? != fs::read(dst.join(rel))? {
                eyre::bail!(
                    "{SHIPPED_DIR}/{} is stale. Run `cargo xtask agent-skill` \
                     and commit the result.",
                    rel.display()
                );
            }
        }
        println!("agent-skill: up to date ({} files)", files.len());
        return Ok(());
    }

    // Remove first, so a file deleted from the source does not linger in the
    // shipped copy — the failure mode a plain overwrite would miss.
    if dst.exists() {
        fs::remove_dir_all(&dst)?;
    }
    for rel in &files {
        let out = dst.join(rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src.join(rel), out)?;
    }
    println!(
        "agent-skill: mirrored {} files into {SHIPPED_DIR}",
        files.len()
    );
    Ok(())
}

/// Collect every file under `dir`, as paths relative to `root`.
fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> eyre::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, out)?;
        } else {
            out.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

/// Build the crate's rustdoc JSON, returning its contents.
fn rustdoc_json(
    project_dir: &Path,
    package: &str,
    stem: &str,
    workspace: bool,
) -> eyre::Result<String> {
    let version = duct::cmd!("rustc", "+nightly", "--version")
        .read()
        .map_err(|e| {
            eyre::eyre!("nightly toolchain is required for rustdoc JSON: {e}. Run `rustup install nightly`.")
        })?;
    println!("api-index: using {}", version.trim());

    let mut args = vec![
        "+nightly".into(),
        "rustdoc".into(),
        "-p".into(),
        package.to_string(),
    ];
    // Cargo refuses `--all-features` for a package outside the workspace.
    if workspace {
        args.push("--all-features".into());
    }
    args.extend(
        ["--", "-Zunstable-options", "--output-format", "json"]
            .iter()
            .map(|a| (*a).to_string()),
    );

    duct::cmd("cargo", &args)
        .dir(project_dir)
        .run()
        .map_err(|e| {
            eyre::eyre!(
                "rustdoc failed: {e}. If this is a `rustc <version> is not supported` error, the \
             installed nightly is below the workspace MSRV — run `rustup update nightly`."
            )
        })?;

    let out: PathBuf = project_dir.join(format!("target/doc/{stem}.json"));
    fs::read_to_string(&out).map_err(|e| {
        eyre::eyre!(
            "rustdoc reported success but {} is missing: {e}",
            out.display()
        )
    })
}

/// One documented public item.
///
/// `module` and `owner` are kept separate rather than folded into a single
/// path, because the emitted document nests type sections *inside* their
/// module. Deriving the module by trimming the last path segment would treat
/// `controller::ErrorDetail::new` as living in a module called
/// `controller::ErrorDetail`, splitting `controller` into many fragments.
struct Item {
    kind: &'static str,
    /// The module the item lives in, e.g. `controller::app_routes`.
    module: String,
    /// The type an inherent method hangs off, if any.
    owner: Option<String>,
    name: String,
    signature: Option<String>,
    /// For a struct, its public fields — the thing you need in order to
    /// construct or destructure it.
    fields: Option<String>,
    summary: Option<String>,
}

impl Item {
    /// Sort key: module, then free items before type sections, then by name.
    fn sort_key(&self) -> (&str, bool, &str, &str) {
        (
            &self.module,
            self.owner.is_some(),
            self.owner.as_deref().unwrap_or(""),
            &self.name,
        )
    }
}

/// Split a full path into its module and final segment.
fn split_path(path: &str) -> (String, String) {
    path.rsplit_once("::").map_or_else(
        || (String::new(), path.to_string()),
        |(m, n)| (m.to_string(), n.to_string()),
    )
}

/// Split a path into `(module, owner, name)`.
///
/// Some items nest under a type rather than directly under a module — enum
/// variants (`model::ModelError::Any`) most visibly. Rust's naming conventions
/// make these unambiguous: modules are `snake_case` (the `non_snake_case` lint
/// enforces it) and types are `PascalCase`, so a leading uppercase on the last
/// module segment means the item belongs in that type's section, not in a
/// module of its own.
fn classify(path: &str) -> (String, Option<String>, String) {
    let (module, name) = split_path(path);
    let (parent, last) = split_path(&module);
    if last.chars().next().is_some_and(char::is_uppercase) {
        (parent, Some(last), name)
    } else {
        (module, None, name)
    }
}

/// Every module rustdoc reports as a real, public module of this crate.
///
/// rustdoc's `paths` map gives each item's *defining* path, which is not always
/// a path a user can name. `model::query::paginate` is a private module whose
/// contents are re-exported from `model::query`, and it appears in no module
/// entry at all — only as a prefix on its items' paths. Publishing that prefix
/// tells a reader to write `model::query::paginate::PaginationQuery`, which
/// fails with "module `paginate` is private": the index producing exactly the
/// invented API it exists to prevent. An eval run caught this.
///
/// So the set of nameable modules is a whitelist, not a blacklist — a segment
/// that never appears as a module is not one.
fn public_modules(paths: &serde_json::Map<String, Value>) -> std::collections::HashSet<String> {
    paths
        .values()
        .filter(|entry| {
            entry.get("crate_id").and_then(Value::as_u64) == Some(0)
                && entry.get("kind").and_then(Value::as_str) == Some("module")
        })
        .map(join_path)
        .collect()
}

/// The deepest ancestor of `module` that is actually a public module.
///
/// Items defined inside a private module are re-exported from somewhere; the
/// nearest public ancestor is where a reader can name them.
fn public_module(module: &str, public: &std::collections::HashSet<String>) -> String {
    let mut segments: Vec<&str> = module.split("::").filter(|s| !s.is_empty()).collect();
    while !segments.is_empty() {
        let candidate = segments.join("::");
        if public.contains(&candidate) {
            return candidate;
        }
        segments.pop();
    }
    String::new()
}

/// Render the whole index, grouped by module.
fn render(doc: &Value, package: &str) -> eyre::Result<String> {
    let paths = doc
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| eyre::eyre!("rustdoc JSON has no `paths` map"))?;
    let index = doc
        .get("index")
        .and_then(Value::as_object)
        .ok_or_else(|| eyre::eyre!("rustdoc JSON has no `index` map"))?;

    let public = public_modules(paths);
    let mut items = top_level_items(paths, index, &public);
    items.extend(trait_methods(paths, index, &public));
    items.extend(inherent_methods(paths, index, &public));

    items.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    items.dedup_by(|a, b| a.sort_key() == b.sort_key() && a.signature == b.signature);

    Ok(emit(&items, &preludes(paths, index), package))
}

/// Structs, enums, traits, free functions and type aliases — everything that
/// has a path of its own.
fn top_level_items(
    paths: &serde_json::Map<String, Value>,
    index: &serde_json::Map<String, Value>,
    public: &std::collections::HashSet<String>,
) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for (id, entry) in paths {
        if entry.get("crate_id").and_then(Value::as_u64) != Some(0) {
            continue;
        }
        let Some(kind) = entry.get("kind").and_then(Value::as_str) else {
            continue;
        };
        let path = join_path(entry);
        if path.is_empty() || skipped(&path) {
            continue;
        }
        // Modules become headings, not entries.
        if kind == "module" {
            continue;
        }
        let detail = index.get(id);
        let (module, owner, name) = classify(&path);
        items.push(Item {
            kind: static_kind(kind),
            module: public_module(&module, public),
            owner,
            name,
            signature: detail.and_then(signature_of),
            fields: detail.and_then(|d| struct_fields(d, index)),
            summary: detail.and_then(summary_of),
        });
    }
    items
}

/// Trait methods. Sea-ORM's entire query surface is traits — `ColumnTrait`,
/// `QueryFilter`, `QueryOrder`, `PaginatorTrait` — so a trait rendered as a
/// bare name teaches nothing. Both eval arms invented `Expr::like` with
/// `ColumnTrait::contains` sitting undocumented one line away. This applies
/// to Loco too: `BackgroundWorker` listed no `perform_later`.
fn trait_methods(
    paths: &serde_json::Map<String, Value>,
    index: &serde_json::Map<String, Value>,
    public: &std::collections::HashSet<String>,
) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for (id, entry) in paths {
        if entry.get("crate_id").and_then(Value::as_u64) != Some(0)
            || entry.get("kind").and_then(Value::as_str) != Some("trait")
        {
            continue;
        }
        let path = join_path(entry);
        if path.is_empty() || skipped(&path) {
            continue;
        }
        let (module, _, trait_name) = classify(&path);
        let Some(members) = index.get(id).and_then(|e| e.pointer("/inner/trait/items")) else {
            continue;
        };
        for member in members.as_array().into_iter().flatten() {
            let Some(detail) = member.as_u64().and_then(|m| index.get(&m.to_string())) else {
                continue;
            };
            let Some(name) = detail.get("name").and_then(Value::as_str) else {
                continue;
            };
            if detail.pointer("/inner/function").is_none() {
                continue;
            }
            items.push(Item {
                kind: "fn",
                module: public_module(&module, public),
                owner: Some(trait_name.clone()),
                name: name.to_string(),
                signature: signature_of(detail),
                fields: None,
                summary: summary_of(detail),
            });
        }
    }
    items
}

/// Associated functions and methods live inside `impl` blocks, which carry
/// no path of their own — attribute each back to the type it is `for`.
fn inherent_methods(
    paths: &serde_json::Map<String, Value>,
    index: &serde_json::Map<String, Value>,
    public: &std::collections::HashSet<String>,
) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for entry in index.values() {
        if entry.get("crate_id").and_then(Value::as_u64) != Some(0) {
            continue;
        }
        let Some(imp) = entry.pointer("/inner/impl") else {
            continue;
        };
        // Trait impls restate the trait's own API; listing them triples the
        // file for no lookup value.
        if imp.get("trait").is_some_and(|t| !t.is_null()) {
            continue;
        }
        let Some(self_ty) = imp.get("for").map(render_type) else {
            continue;
        };
        let Some(owner) = paths
            .get(&type_id(imp.get("for")).unwrap_or_default())
            .map(join_path)
            .filter(|p| !p.is_empty())
        else {
            continue;
        };
        if skipped(&owner) {
            continue;
        }
        for member in imp
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(detail) = member.as_u64().and_then(|m| index.get(&m.to_string())) else {
                continue;
            };
            if detail.get("visibility").and_then(Value::as_str) != Some("public") {
                continue;
            }
            let Some(name) = detail.get("name").and_then(Value::as_str) else {
                continue;
            };
            if detail.pointer("/inner/function").is_none() {
                continue;
            }
            let (module, _) = split_path(&owner);
            items.push(Item {
                kind: "fn",
                module: public_module(&module, public),
                owner: Some(self_ty.clone()),
                name: name.to_string(),
                signature: signature_of(detail),
                fields: None,
                summary: summary_of(detail),
            });
        }
    }
    items
}

/// Every `prelude` module and the names its glob import brings into scope.
///
/// Preludes are pure re-export modules, so they carry no items of their own and
/// would otherwise render as empty headings — yet `use loco_rs::prelude::*` is
/// the first line of essentially every file in a Loco app. Resolving the
/// re-exports is what tells a reader which names are *already* in scope.
fn preludes(
    paths: &serde_json::Map<String, Value>,
    index: &serde_json::Map<String, Value>,
) -> Vec<(String, Vec<String>)> {
    let mut found: Vec<(String, Vec<String>)> = paths
        .iter()
        .filter(|(_, entry)| {
            entry.get("crate_id").and_then(Value::as_u64) == Some(0)
                && entry.get("kind").and_then(Value::as_str) == Some("module")
        })
        .filter_map(|(id, entry)| {
            let path = join_path(entry);
            if !path.ends_with("prelude") {
                return None;
            }
            let members = index
                .get(id)?
                .pointer("/inner/module/items")?
                .as_array()?
                .iter()
                .filter_map(|m| index.get(&m.as_u64()?.to_string()))
                .filter_map(|m| m.pointer("/inner/use/name")?.as_str().map(str::to_string))
                .collect::<Vec<_>>();
            if members.is_empty() {
                return None;
            }
            Some((path, members))
        })
        .collect();

    for (_, members) in &mut found {
        members.sort();
        members.dedup();
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Format the collected items as grouped markdown.
fn emit(items: &[Item], preludes: &[(String, Vec<String>)], package: &str) -> String {
    let mut out = format!(
        "# `{package}` public API index\n\n\
         GENERATED by `cargo xtask api-index` from rustdoc JSON. Do not edit by hand.\n\n\
         Every public symbol in `{package}`, grouped by module. If a symbol is not\n\
         here, it does not exist — do not import it. Inherent methods and trait\n\
         methods are listed under their type; trait *implementations* are omitted,\n\
         since they restate the trait's own API.\n"
    );

    for (path, members) in preludes {
        let _ = write!(
            out,
            "\n## `loco_rs::{path}`\n\n\
             `use loco_rs::{path}::*;` brings these into scope. They are already \
             available — importing them again is redundant.\n\n{}\n",
            members
                .iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut current_module: Option<&str> = None;
    let mut current_owner: Option<&str> = None;
    for item in items {
        let module = if item.module.is_empty() {
            "loco_rs"
        } else {
            &item.module
        };
        if current_module != Some(module) {
            let _ = write!(out, "\n## `{module}`\n");
            current_module = Some(module);
            current_owner = None;
        }
        if current_owner != item.owner.as_deref() {
            if let Some(owner) = &item.owner {
                let _ = write!(out, "\n\n### `{owner}`\n");
            }
            current_owner = item.owner.as_deref();
        }
        let _ = write!(out, "\n- `{}` **{}**", item.kind, item.name);
        if let Some(sig) = &item.signature {
            let _ = write!(out, "`{sig}`");
        }
        if let Some(fields) = &item.fields {
            let _ = write!(out, "` {fields}`");
        }
        if let Some(summary) = &item.summary {
            let _ = write!(out, " — {summary}");
        }
    }
    out.push('\n');
    out
}

/// `true` when a path is internal surface an app author never names.
fn skipped(path: &str) -> bool {
    SKIP_PREFIXES.iter().any(|p| path.starts_with(p)) || path.contains("::_")
}

/// rustdoc kind strings, narrowed to the ones we emit.
fn static_kind(kind: &str) -> &'static str {
    match kind {
        "struct" => "struct",
        "enum" => "enum",
        "trait" => "trait",
        "function" => "fn",
        "static" => "static",
        "macro" => "macro",
        "variant" => "variant",
        "struct_field" => "field",
        // Free and associated forms collapse to one label — a reader of the
        // index does not care which rustdoc called it.
        "type_alias" | "assoc_type" => "type",
        "constant" | "assoc_const" => "const",
        _ => "item",
    }
}

/// `["loco_rs", "controller", "format"]` → `controller::format`.
///
/// The crate name is dropped: every path in this file is `loco_rs`-rooted, and
/// repeating it 850 times buys nothing.
fn join_path(entry: &Value) -> String {
    entry
        .get("path")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(Value::as_str)
                .skip(1)
                .collect::<Vec<_>>()
                .join("::")
        })
        .unwrap_or_default()
}

/// A struct's public fields, rendered as `{ name: Type, ... }`.
///
/// Without these the index names a type but says nothing about constructing or
/// destructuring it. Two eval submissions invented `PageResponse.total_pages`
/// and `PageResponse.total_items`; the real shape is `{ page, meta }`, with the
/// counts living on `meta`. Nothing in the index contradicted them.
fn struct_fields(entry: &Value, index: &serde_json::Map<String, Value>) -> Option<String> {
    let ids = entry
        .pointer("/inner/struct/kind/plain/fields")?
        .as_array()?;
    let rendered = ids
        .iter()
        .filter_map(|id| {
            let field = index.get(&id.as_u64()?.to_string())?;
            if field.get("visibility").and_then(Value::as_str) != Some("public") {
                return None;
            }
            let name = field.get("name")?.as_str()?;
            let ty = render_type(field.pointer("/inner/struct_field")?);
            Some(format!("{name}: {ty}"))
        })
        .collect::<Vec<_>>();
    (!rendered.is_empty()).then(|| format!("{{ {} }}", rendered.join(", ")))
}

/// First sentence of an item's doc comment, if it has one.
fn summary_of(entry: &Value) -> Option<String> {
    let docs = entry.get("docs").and_then(Value::as_str)?;
    let first = docs.lines().find(|l| !l.trim().is_empty())?.trim();
    if first.is_empty() {
        return None;
    }
    Some(first.trim_end_matches('.').to_string())
}

/// Reconstruct a callable's parameter list and return type.
fn signature_of(entry: &Value) -> Option<String> {
    let sig = entry.pointer("/inner/function/sig")?;
    let inputs = sig
        .get("inputs")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(|arg| {
                    let pair = arg.as_array()?;
                    let name = pair.first()?.as_str()?;
                    let ty = render_type(pair.get(1)?);
                    Some(if name == "self" && ty == "self" {
                        "self".to_string()
                    } else {
                        format!("{name}: {ty}")
                    })
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let output = sig
        .get("output")
        .filter(|o| !o.is_null())
        .map(render_type)
        .map_or(String::new(), |t| format!(" -> {t}"));

    // Without the where-clause a generic parameter is just a bare letter. Two
    // independent eval submissions failed to compile against
    // `Cache::get_or_insert_with_expiry` because `f: F` said nothing about F
    // being a Future with a particular Output — the one fact needed to call it.
    let bounds = entry
        .pointer("/inner/function/generics")
        .map(render_where)
        .filter(|w| !w.is_empty())
        .map_or(String::new(), |w| format!(" where {w}"));

    Some(format!("({inputs}){output}{bounds}"))
}

/// Render a `where` clause from a rustdoc `generics` block.
fn render_where(generics: &Value) -> String {
    let predicates = generics
        .get("where_predicates")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    predicates
        .iter()
        .filter_map(|predicate| {
            let bound = predicate.get("bound_predicate")?;
            let subject = bound.get("type").map(render_type)?;
            let traits = bound
                .get("bounds")
                .and_then(Value::as_array)?
                .iter()
                .filter_map(render_trait_bound)
                .collect::<Vec<_>>();
            (!traits.is_empty()).then(|| format!("{subject}: {}", traits.join(" + ")))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// One trait bound, including its generic arguments and associated types.
fn render_trait_bound(bound: &Value) -> Option<String> {
    let path = bound.pointer("/trait_bound/trait")?;
    let name = path
        .get("path")
        .and_then(Value::as_str)?
        .rsplit("::")
        .next()
        .unwrap_or_default()
        .to_string();

    let angle = path.pointer("/args/angle_bracketed");
    let args = angle
        .and_then(|a| a.get("args"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|a| a.get("type").map(render_type))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // `Future<Output = X>` lives in `constraints`, not `args` — and Output is
    // the entire point of naming the bound.
    let constraints = angle
        .and_then(|a| a.get("constraints"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?;
                    let ty = c.pointer("/binding/equality/type").map(render_type)?;
                    Some(format!("{name} = {ty}"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let all = [args, constraints].concat();
    Some(if all.is_empty() {
        name
    } else {
        format!("{name}<{}>", all.join(", "))
    })
}

/// The `id` a type resolves to, when it is a named path.
fn type_id(ty: Option<&Value>) -> Option<String> {
    ty?.pointer("/resolved_path/id")
        .and_then(Value::as_u64)
        .map(|id| id.to_string())
}

/// Render a rustdoc `Type` back into something that reads like Rust source.
///
/// Full fidelity is not the goal — this is a lookup aid, so generic arguments
/// are rendered but lifetimes and where-clauses are dropped for legibility.
fn render_type(ty: &Value) -> String {
    if let Some(p) = ty.get("primitive").and_then(Value::as_str) {
        return p.to_string();
    }
    if let Some(g) = ty.get("generic").and_then(Value::as_str) {
        return g.to_string();
    }
    if let Some(path) = ty.get("resolved_path") {
        let name = path
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .rsplit("::")
            .next()
            .unwrap_or("?");
        let args = path
            .pointer("/args/angle_bracketed/args")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|a| a.get("type").map(render_type))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return if args.is_empty() {
            name.to_string()
        } else {
            format!("{name}<{}>", args.join(", "))
        };
    }
    if let Some(r) = ty.get("borrowed_ref") {
        let inner = r.get("type").map_or_else(|| "?".to_string(), render_type);
        let mutable = r
            .get("is_mutable")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        return if mutable {
            format!("&mut {inner}")
        } else {
            format!("&{inner}")
        };
    }
    if let Some(items) = ty.get("tuple").and_then(Value::as_array) {
        if items.is_empty() {
            return "()".to_string();
        }
        let rendered = items.iter().map(render_type).collect::<Vec<_>>();
        return format!("({})", rendered.join(", "));
    }
    if let Some(inner) = ty.get("slice") {
        return format!("[{}]", render_type(inner));
    }
    if let Some(arr) = ty.get("array") {
        let inner = arr.get("type").map_or_else(|| "?".to_string(), render_type);
        let len = arr.get("len").and_then(Value::as_str).unwrap_or("_");
        return format!("[{inner}; {len}]");
    }
    if ty.get("impl_trait").is_some() {
        return "impl Trait".to_string();
    }
    if let Some(d) = ty.pointer("/dyn_trait/traits") {
        let names = d
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|t| {
                        t.pointer("/trait/path")
                            .and_then(Value::as_str)
                            .map(|p| p.rsplit("::").next().unwrap_or(p).to_string())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return format!("dyn {}", names.join(" + "));
    }
    if let Some(q) = ty.get("qualified_path") {
        let name = q.get("name").and_then(Value::as_str).unwrap_or("?");
        let base = q
            .get("self_type")
            .map_or_else(|| "?".to_string(), render_type);
        return format!("{base}::{name}");
    }
    if ty.get("infer").is_some() {
        return "_".to_string();
    }
    if let Some(ptr) = ty.get("raw_pointer") {
        let inner = ptr.get("type").map_or_else(|| "?".to_string(), render_type);
        return format!("*{inner}");
    }
    // `self` in a method signature arrives as a bare string.
    ty.as_str().unwrap_or("_").to_string()
}
