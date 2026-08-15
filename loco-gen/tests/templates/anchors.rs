//! Every app tree that ships in this repository must stay generatable into.
//!
//! Generators wire their output into the app by injecting at anchor comments —
//! `inject-above` in the migrator, `// scaffold:imports` and
//! `// scaffold:routes` in the SPA route table. An anchor that goes missing
//! used to cost nothing at review time and everything at use time: through
//! rrgen 0.5 the injection rewrote the file unchanged and still reported
//! `injected: …`, so the generated code was written, compiled, and never
//! reached.
//!
//! rrgen 0.6 makes that a hard error, which is the right behaviour for a user's
//! own app and the wrong thing to discover in ours: `examples/reference_spa` had
//! lost both scaffold anchors, so `loco g scaffold` inside the repository's own
//! reference SPA would have failed outright.
//!
//! These tests read the files as they ship. Nothing else does.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("loco-gen sits one level below the repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Both the template new apps are built from and the example apps users clone.
const MIGRATORS: [&str; 3] = [
    "loco-new/base_template/migration/src/lib.rs.t",
    "examples/demo/migration/src/lib.rs",
    "examples/reference_spa/migration/src/lib.rs",
];

const ROUTE_TABLES: [&str; 2] = [
    "loco-new/base_template/frontend/src/routes.tsx",
    "examples/reference_spa/frontend/src/routes.tsx",
];

#[test]
fn every_migrator_keeps_the_anchor_migrations_are_registered_at() {
    for migrator in MIGRATORS {
        let content = read(migrator);
        assert!(
            content.contains("inject-above"),
            "{migrator} has no `inject-above` comment, so `loco g model` cannot register \
             a migration in it"
        );
    }
}

#[test]
fn every_route_table_keeps_the_anchors_the_scaffold_injects_at() {
    for route_table in ROUTE_TABLES {
        let content = read(route_table);
        for anchor in ["// scaffold:imports", "// scaffold:routes"] {
            assert!(
                content.contains(anchor),
                "{route_table} has no `{anchor}` comment, so `loco g scaffold` cannot add \
                 a page to it"
            );
        }
    }
}
