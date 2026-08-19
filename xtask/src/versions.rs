use std::{
    env::{self, current_dir},
    path::Path,
};

use cargo_metadata::semver::Version;
use duct::cmd;
use regex::Regex;

use crate::{
    ci::{cargo_clippy, cargo_fmt},
    errors::{Error, Result},
};

/// Every version this command rewrites, as `(file, pattern)`.
///
/// These are named so `patterns_still_match_the_tree` can prove each one still
/// finds its target. A pattern that quietly stops matching — because the line
/// it anchors to was reworded, or because it was pinned to a literal version —
/// is invisible during a release and only surfaces in a published artifact.
const CRATE_VERSION: (&str, &str) = ("Cargo.toml", r"(?m)^version.*$");
const LOCO_GEN_VERSION: (&str, &str) = ("loco-gen/Cargo.toml", r"(?m)^version.*$");
const LOCO_GEN_DEP: (&str, &str) = ("Cargo.toml", r"(?m)^loco-gen [^,]*,");
// The CLI crate publishes as `loco` and is listed in the runbook this command
// prints, but it was not in this table until 1.1.0 — so `bump` left it at the
// previous version while bumping everything around it. `cargo publish` then
// rejects it outright ("crate version already uploaded"), and the version a
// user sees from `loco --version` names a release the binary is not.
const LOCO_NEW_VERSION: (&str, &str) = ("loco-new/Cargo.toml", r"(?m)^version.*$");
const LOCO_VERSION_CONST: (&str, &str) = (
    "loco-new/src/lib.rs",
    r#"(?m)^pub const LOCO_VERSION: &str = "[^"]*";$"#,
);

/// Rewrites a version in place, and **fails if it cannot**.
///
/// A missing file or a pattern that no longer matches means the release is
/// about to ship a half-bumped tree, so both are errors. Reporting them and
/// carrying on is how `LOCO_VERSION` sat at a stale value for release after
/// release: its pattern had been pinned to a long-gone version, so every run
/// printed one line and moved on.
fn bump_version_in_file(
    file_path: &str,
    version_regex: &str,
    replacement_version: &str,
    once: bool,
) -> Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(Error::Message(format!("{file_path} does not exist")));
    }

    println!("bumping in {file_path}");
    let file_content = std::fs::read_to_string(path)?;

    let re = Regex::new(version_regex).expect("Invalid regex");
    if !re.is_match(&file_content) {
        return Err(Error::Message(format!(
            "`{version_regex}` matched nothing in {file_path}, so its version was not \
             bumped. Fix the pattern or the file before releasing."
        )));
    }

    let new_content = if once {
        re.replace(&file_content, replacement_version)
    } else {
        re.replace_all(&file_content, replacement_version)
    };

    std::fs::write(path, new_content.to_string())?;
    Ok(())
}

/// Bump every version a release touches.
///
/// # Errors
/// when a `loco-new` fmt/clippy pre-check fails, a tracked file is missing or
/// unreadable, or a version pattern no longer matches the file it bumps
pub fn bump_version(version: &Version) -> Result<()> {
    // testing loco-new will test 4 combinations of starters
    // sets LOCO_DEV_MODE_PATH=/<path-to>/projects/loco/ and shared cargo build path
    let new_path = Path::new("loco-new");
    cargo_fmt(new_path)?;
    cargo_clippy(new_path)?;
    if env::var("LOCO_DEV_MODE_PATH").is_err() {
        let loco_path = current_dir()?.to_string_lossy().to_string();
        println!("setting LOCO_DEV_MODE_PATH to `{loco_path}`");
        // SAFETY: single-threaded xtask CLI; set before the child cargo process is spawned.
        unsafe { env::set_var("LOCO_DEV_MODE_PATH", loco_path) };

        // this should accelerate starters compilation
        println!("setting CARGO_SHARED_PATH");
        // SAFETY: single-threaded xtask CLI; set before the child cargo process is spawned.
        unsafe { env::set_var("CARGO_SHARED_PATH", "/tmp/cargo-shared-path") };
    }

    cmd("cargo", ["test", "--", "--test-threads", "1"].as_slice())
        .dir(new_path)
        .run()?;
    // SAFETY: single-threaded xtask CLI; set before the child cargo process is spawned.
    unsafe { env::remove_var("CARGO_SHARED_PATH") };

    // replace main versions
    let version_replacement = format!(r#"version = "{version}""#);
    bump_version_in_file(CRATE_VERSION.0, CRATE_VERSION.1, &version_replacement, true)?;
    bump_version_in_file(
        LOCO_GEN_VERSION.0,
        LOCO_GEN_VERSION.1,
        &version_replacement,
        true,
    )?;
    bump_version_in_file(
        LOCO_NEW_VERSION.0,
        LOCO_NEW_VERSION.1,
        &version_replacement,
        true,
    )?;

    // sync new version to subcrates in main Cargo.toml
    let loco_gen_dep = format!(r#"loco-gen = {{ version = "{version}","#);
    bump_version_in_file(LOCO_GEN_DEP.0, LOCO_GEN_DEP.1, &loco_gen_dep, false)?;

    // The `loco-rs` requirement stamped into generated apps. It carries only
    // `major.minor` — a compatibility floor, not the exact release.
    let const_version_replacement = format!(
        r#"pub const LOCO_VERSION: &str = "{}.{}";"#,
        version.major, version.minor
    );
    bump_version_in_file(
        LOCO_VERSION_CONST.0,
        LOCO_VERSION_CONST.1,
        &const_version_replacement,
        true,
    )?;

    println!(
        "
    PUBLISHING

    Order matters: `loco-rs` depends on `loco-gen` by version, so loco-gen has
    to be on the index before loco-rs will resolve. Publishing out of order
    fails at the second step, not the first.

    = framework =

    $ cd loco-gen && cargo publish
    $ cargo publish

    = loco 'new' CLI =

    $ cd loco-new && cargo publish

    = docs =

    The site is built from `website/` (Astro/Starlight) and deployed by the
    hosting provider from the default branch — there is no publish command
    here. Verify locally first:

    $ cd website && pnpm install --frozen-lockfile && pnpm test && pnpm run check && pnpm run build
    "
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CRATE_VERSION, LOCO_GEN_DEP, LOCO_GEN_VERSION, LOCO_NEW_VERSION, LOCO_VERSION_CONST,
    };

    /// Each bump pattern must still find its target in the tree.
    ///
    /// Releases are infrequent, so a rotted pattern hides for months: the
    /// `LOCO_VERSION` one was pinned to a literal `"0.13"` and matched nothing
    /// from 0.14 onward, leaving generated apps requiring a `loco-rs` too old
    /// to read the config they shipped with.
    #[test]
    fn patterns_still_match_the_tree() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask sits inside the repo");

        for (file, pattern) in [
            CRATE_VERSION,
            LOCO_GEN_VERSION,
            LOCO_NEW_VERSION,
            LOCO_GEN_DEP,
            LOCO_VERSION_CONST,
        ] {
            let content = std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|e| panic!("`cargo xtask bump` rewrites {file}, but: {e}"));

            assert!(
                regex::Regex::new(pattern)
                    .expect("bump pattern is a valid regex")
                    .is_match(&content),
                "`cargo xtask bump` rewrites {file} with `{pattern}`, which no longer \
                 matches anything there. A release would leave that version stale \
                 without saying so."
            );
        }
    }
}
