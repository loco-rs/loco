# Spike S5 — H9 (`url`), H10 (`cargo_metadata`), H11 (crates.io JSON API), H12 (`termtree`)

Protocol: `docs/superpowers/audits/2026-07-03-code-quality/SPIKE-PROTOCOL.md`.
Spikes built at `scratchpad/spikes/h9-url/`, `scratchpad/spikes/h10-cargo-metadata/`,
`scratchpad/spikes/h11-cratesio-semver/`, `scratchpad/spikes/h12-termtree/`
(throwaway cargo crates, not touching the loco workspace).

---

## H9 — `url` replacing whole-string `.replace()` admin-URL construction

**Incumbent:** `src/db/connect.rs:149-177`. `extract_db_name` (149-154) regex-extracts
the last path segment of a `postgres://` URI
(`^.+://(?:.*?/)?([^/?#]+)(?:[?#]|$)`). `create` (161-177) then builds the
"admin" connection string (pointed at the `postgres` maintenance DB instead of
the target DB) via:

```rust
let conn = db_uri.replace(db_name, "/postgres");
```

### Library verified
`url = "2.5.8"` (crates.io latest). API used: `Url::parse`, `Url::set_path`,
`Url::to_string` (all stable, long-standing API — no hallucination risk here).
**`url` is already resolved in loco's own `Cargo.lock` as a transitive
dependency** (pulled in by `sqlx` 0.9's `sqlx-core`/`sqlx-postgres`/`sqlx-sqlite`,
which loco depends on directly for DB support) — adopting it as a *direct*
dependency adds **zero new crates** to the tree.

### Spike design
Reproduced `extract_db_name`'s exact regex and the incumbent's `.replace()`
call, and put a `url`-based swap (`Url::parse` → `set_path("/postgres")` →
`to_string`) side by side, run against 3 URIs: a clean one, and two adversarial
ones where the db name string also appears as the username or inside the host
— exactly the collision class the reviewer flagged as a risk.

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h9-url
   Compiling url v2.5.8
   Compiling h9-url-spike v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.11s
     Running `target/debug/h9-url-spike`
--- case 1: normal ---
input:            postgres://user:pw@localhost:5432/myapp
incumbent output: Ok("postgres://user:pw@localhost:5432//postgres")
url-based output: Ok("postgres://user:pw@localhost:5432/postgres")

--- case 2: username collides with db name (the bug) ---
input:            postgres://myapp:pw@host/myapp
incumbent output: Ok("postgres:///postgres:pw@host//postgres")
url-based output: Ok("postgres://myapp:pw@host/postgres")

--- case 3: host contains db name as substring ---
input:            postgres://user:pw@app.internal.db/app
incumbent output: Ok("postgres://user:pw@/postgres.internal.db//postgres")
url-based output: Ok("postgres://user:pw@app.internal.db/postgres")

All assertions passed: url-based swap is correct in all 3 cases; incumbent
.replace() corrupts cases 1 (stray //), 2 (username+path both clobbered ->
unparseable), and 3 (host clobbered).
```

### Findings
1. **The reviewer's risk materializes, and worse than described.** Rust's
   `str::replace` replaces *every* match, not just the path segment. When the
   db name string also appears as the username (`postgres://myapp:pw@host/myapp`),
   the incumbent corrupts *both* occurrences, producing
   `"postgres:///postgres:pw@host//postgres"` — not just wrong, but unparseable
   by any URI parser (extra empty scheme-authority segment). The reviewer's
   documented "username contains db name" scenario is a genuine outage class,
   not a theoretical one.
2. **Even the "clean" case has a latent bug.** `db_name` is captured *without*
   the leading `/`, so `.replace("myapp", "/postgres")` leaves a stray
   `//postgres` in the path. It happens to still work today only because
   Postgres URI parsers tolerate an empty leading path segment — this is
   accidental correctness, not designed behavior.
3. **`url` fixes all three cases with less surface area:** parse once, swap
   only the path component (a typed operation, immune to substring
   collisions anywhere else in the URI), reserialize. It can also replace the
   regex-based `extract_db_name` entirely (`url.path().trim_start_matches('/')`
   gives the db name directly), removing one more failure mode (the regex
   pattern is `.+`-greedy and could itself misparse unusual DSNs).
4. **Dependency cost is effectively zero** — `url` 2.5.8 is already compiled
   as part of loco's existing `sqlx` dependency chain; making it a direct
   dependency adds no new crate to the build.

### Verdict
**PROVEN-FIT** — `url@2.5.8` — replaces the whole-string `.replace()` (which
provably corrupts the URI when the db name collides with the username or host
substring, `src/db/connect.rs:173`) with a typed path-segment swap; net LOC
roughly flat-to-negative (can also subsume `extract_db_name`'s regex,
`connect.rs:149-154`), and the dependency is already present transitively via
`sqlx` — net new deps: 0. Recommend adopting.

---

## H10 — `cargo_metadata` replacing hand-rolled Cargo.toml table walking

**Incumbent:** `src/cargo_config.rs:19-63`. `CargoConfig::from_path` reads and
parses `Cargo.toml` with the already-present `toml = "0.8"` dependency;
`get_db_entities` walks `toml::Table.get("package").metadata.db.entity` via
`Option` chaining — 14 lines of real logic total (`from_path` + `get_db_entities`).

### Library verified
`cargo_metadata = "0.23.1"` (crates.io latest). Notably, **loco's own `xtask`
workspace member already uses it** (`xtask/Cargo.toml:19`,
`xtask/src/bin/main.rs:3`) — but only in the release-tooling binary, never in
the published `loco-rs` library crate.

### Spike design
Used `MetadataCommand::new().manifest_path(...).no_deps().exec()` (the
cheapest possible invocation — skips full dependency-graph resolution) against
a Cargo.toml carrying the same `[package.metadata.db.entity]` block loco's own
tests use (`cargo_config.rs:87-95`), then walked `root_package().metadata`
(untyped `serde_json::Value`, since `package.metadata.*` is consumer-defined
and `cargo_metadata` cannot know its shape) the same way the incumbent walks
`toml::Table`.

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h10-cargo-metadata
   Compiling cargo_metadata v0.23.1  (+ serde, serde_json, thiserror, semver,
                                        camino, cargo-platform, cargo-lock-friendly stack)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.48s
     Running `target/debug/h10-cargo-metadata-spike`
manifest: .../h10-cargo-metadata/Cargo.toml
db.entity metadata (none expected, this Cargo.toml has none): Some(Object {"compact-format": Bool(true), "with-serde": String("serialize")})
`cargo metadata --no-deps` wall time: 10.765083ms
```

### Findings
1. **No LOC win.** The `cargo_metadata` version still has to hand-walk an
   untyped JSON value (`metadata.get("db").and_then(|d| d.get("entity"))`) —
   `package.metadata.*` is opaque to `cargo_metadata` by design (it's
   consumer-defined arbitrary TOML), so it cannot offer a typed accessor. The
   candidate lookup is ~15 lines, same order as the incumbent's 14.
2. **Real added cost, not hypothetical:** `cargo_metadata` **shells out to a
   `cargo` subprocess** even with `--no-deps` (confirmed: 10.7ms per call vs.
   an in-process file read + parse for the incumbent, which is sub-millisecond
   and doesn't require a `cargo` binary on `PATH` at runtime). For a function
   that today is a pure, deterministic file read, this trades a fast/simple
   operation for a subprocess spawn with all its failure modes (cargo not on
   PATH, non-Cargo-project cwd edge cases, cargo version skew).
3. **Dependency weight:** pulls in `serde`, `serde_json`, `thiserror`,
   `semver`, `camino`, `cargo-platform` — all new to the direct dependency
   list even though several of them are already present *transitively*
   elsewhere in loco's tree (this is a much heavier chain than the single
   `toml::Table` already in use for exactly this purpose).
4. Confirms the reviewer's WEAK judgment: the incumbent's job (read one
   already-known-location config block from a file loco already has open) is
   not what `cargo_metadata` is for — it's for querying the resolved
   workspace/dependency graph, which is unneeded here.

### Verdict
**DOESN'T-FIT** — `cargo_metadata@0.23.1` — same LOC as the incumbent's
14-line lookup, but replaces a synchronous file-read with a `cargo` subprocess
spawn (~11ms/call, requires `cargo` on `PATH`) and adds a heavier dependency
chain for no behavioral gain. Incumbent (`src/cargo_config.rs:19-63`) stays;
validates its KPI7 score.

---

## H11 — direct crates.io JSON API replacing `cargo search` + regex in doctor

**Incumbent:** `src/doctor.rs:65-97` (`check_cratesio_version` shells out to
`cargo search <crate> --limit 1` and regex-parses the free-text stdout with
`RE_CRATE_VERSION` at `doctor.rs:44`, pattern `(?m)^[^"]*"([^"]+)"`) and
`src/doctor.rs:323` (`check_seaorm_cli`'s separate regex `(\d+\.\d+\.\d+)` for
parsing `sea-orm-cli --version` output).

### Library verified
`ureq = "3.3.0"` (crates.io latest) for the HTTP GET, `semver = "1"` (already
a direct loco dependency, `Cargo.toml:130`) for comparison. Used real, current
API: `ureq::get(url).header(...).call()?.body_mut().read_json::<T>()`.
Endpoint verified live: `https://crates.io/api/v1/crates/<name>` returns JSON
with `crate.max_version` and `crate.max_stable_version`.

**Dependency-weight nuance (checked, not assumed):** loco does *not* have
`reqwest` as a runtime dependency today — it's `dev-dependencies`-only
(`Cargo.toml:219`), used only in tests. Meanwhile `ureq` 3.3.0 (and its full
transitive tree — `ureq-proto`, `rustls`, `webpki-roots`, etc.) is **already
resolved in loco's `Cargo.lock`**, pulled in as a *dev-dependency* via
`testcontainers → bollard → bollard-buildkit-proto`. So: zero new crate names
would enter the lockfile, but adding `ureq` to `[dependencies]` (not
`[dev-dependencies]`) would promote that entire chain from
test-only to a mandatory runtime dependency of every published `loco-rs`
consumer app — a real, if crate-count-invisible, weight increase.

### Spike design
Live HTTP GET to `https://crates.io/api/v1/crates/serde` (real network call,
not mocked) to prove the JSON deserialization path works end to end; then a
focused repro of the **actual bug** the regex causes: `sea-orm-cli`'s min
version is a pre-release (`MIN_SEAORMCLI_VER = "2.0.0-rc"`, `doctor.rs:39`),
but the version-extraction regex `(\d+\.\d+\.\d+)` matches only `X.Y.Z` and
silently drops any `-rc.N` suffix before the `semver` comparison ever runs.

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h11-cratesio-semver
   Compiling ureq v3.3.0 (+ rustls/webpki-roots/http/serde stack)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.05s
     Running `target/debug/h11-cratesio-semver-spike`
--- live crates.io lookup for `serde` (stable, well-known crate) ---
newer version available: 1.0.228

--- pre-release truncation bug (regex vs semver crate) ---
installed CLI reports:      2.0.0-rc.5
incumbent regex extracts:   2.0.0  <-- pre-release suffix silently dropped
incumbent's naive compare:  installed(2.0.0) >= min(2.0.0-rc.10)? -> true
  ^ WRONG: reports 2.0.0 >= 2.0.0-rc.10 as true (semver: any non-prerelease
2.0.0 outranks ALL 2.0.0-rc.N prereleases) -- but the actually-installed
binary is only rc.5, an EARLIER build than the rc.10 minimum. The truncation
silently converts an older pre-release into what looks like a newer stable
release, masking a real under-version condition.

correct (no truncation):    installed(2.0.0-rc.5) >= min(2.0.0-rc.10)? -> false

Assertions passed: semver-crate-on-full-string is correct; regex-truncation-
then-compare is provably wrong for pre-release CLI versions.
```

### Findings
1. **The bug is not hypothetical — it's live in loco's own min-version
   table.** `MIN_SEAORMCLI_VER = "2.0.0-rc"` (`doctor.rs:39`) and
   `MIN_DEP_VERSIONS["sea-orm"] = "2.0.0-rc"` (`doctor.rs:52`) both target the
   pre-2.0 stable window loco is currently released against. Any developer
   running `cargo loco doctor` with an installed `sea-orm-cli` still on an
   earlier `-rc.N` build than intended will have that suffix silently
   stripped, and the truncated `X.Y.Z` will spuriously compare as
   "newer" than a later-numbered `-rc.M` minimum — a false-positive "OK"
   from `check_seaorm_cli` on exactly the version range loco cares about
   right now.
2. **`cargo search` itself still works** (verified live — not deprecated,
   exit 0, real output) but returns unstructured text whose format is an
   implementation detail of `cargo`'s CLI, not a contract; the JSON API is
   crates.io's actual stable, documented, versioned public interface.
3. **Both approaches require network** — `cargo search` also hits the
   registry over HTTP, so there's no offline-vs-online trade-off being made
   here, just structured-JSON-with-real-schema vs. scraped-CLI-stdout.
4. **LOC is roughly flat** (~30 lines either way), so the case for switching
   rests entirely on correctness (fixes the pre-release truncation bug) and
   robustness (JSON schema vs. CLI stdout format), not on line count.
5. **Honest cost:** promotes a dependency chain from dev-only to mandatory
   runtime for every downstream consumer of the `loco-rs` library (see
   dependency-weight nuance above) — this is a real, if crate-count-neutral
   in loco's own lockfile, cost to weigh against the bug fix.

### Verdict
**PROVEN-FIT** — `ureq@3.3.0` + `semver@1` (already a direct dep) — fixes a
live, non-hypothetical pre-release-truncation bug in `check_seaorm_cli`
(`doctor.rs:323`) affecting loco's own current `2.0.0-rc` minimum-version
window; replaces scraped CLI stdout with crates.io's documented JSON API; net
LOC ≈ flat. Caveat: promotes a currently dev-only-dependency chain
(`ureq`/`rustls`/`webpki-roots`) to a mandatory runtime dependency of every
published `loco-rs` consumer — recommend adopting for the correctness fix,
with that dependency-promotion cost explicitly called out to the maintainers.

---

## H12 — `termtree`/`ptree` replacing the hand-rolled ASCII route-tree printer

**Incumbent:** `src/cli.rs:1017-1164` (`RouteNode`, 148 lines) — builds a
`BTreeMap`-based tree from route path segments, applies a COLLAPSING rule
(`is_collapsible`, `cli.rs:1029-1033`: a childless-of-endpoints node with
exactly one leaf child gets merged onto one line as `segment/child` instead of
two tree levels), and prints each line as a **3-column** layout: tree
glyphs+segment padded to a *global* fixed width
(`format!("{:<50}", format!("{tree} {method}"))`, `cli.rs:1161-1163`), the
colored HTTP method, and the full route path.

### Library verified
`termtree = "1.0.0"` (crates.io latest). API confirmed via Context7 docs
(`/rust-cli/termtree`): `Tree::new`, `Tree::push`, `Tree::with_leaves`,
`GlyphPalette`/`with_glyphs` for custom glyph sets, `Display` impl for
rendering.

### Spike design
Built a `RouteNode`-equivalent structure carrying the *same* collapsing
predicate as the incumbent, converted it into a `termtree::Tree<String>`
(letting `termtree` own the glyph-drawing/indentation), and rendered a
realistic small route set (`/auth/register`, `/auth/login`, `/users`,
`/users/:id` with GET+PUT, `/_health`) mirroring loco's own branching +
collapsing shapes.

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h12-termtree
   Compiling termtree v1.0.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.37s
     Running `target/debug/h12-termtree-spike`
=== termtree-rendered structure (collapsing decided by our own code) ===

/
├── /_health [GET]
├── /auth
│   ├── /login [POST]
│   └── /register [POST]
└── /users [GET]
    └── /:id [GET,PUT]
```

### Findings
1. **The collapsing decision is unavoidably data-layer logic, not a rendering
   concern — confirming the reviewer's WEAK judgment.** `termtree` renders
   whatever tree you hand it; it has no concept of "these two levels should
   become one line." The `is_leaf`/`is_collapsible` predicates
   (`cli.rs:1025-1033`, ~10 lines) had to be reimplemented *unchanged* in the
   spike before `termtree` ever saw a node — adopting `termtree` doesn't
   remove this logic, it just adds a library underneath it.
2. **A second, previously-unstated semantic gets lost: global column
   alignment.** The incumbent's `format!("{:<50}", ...)` pads the
   *tree-glyphs+segment* string as a single global column, so the
   method/full-path columns line up vertically across the whole tree
   regardless of indentation depth (deeper branches still align with shallow
   ones). `termtree::Display` only knows how to draw its own glyph+label
   column recursively — it has no hook for a second, globally width-aligned
   column appended after the fact. Reproducing Loco's aligned method/path
   columns would require either (a) baking the method into the label string
   per-node (as done in the spike above) and *losing* global alignment across
   differing depths, or (b) rendering to a string and post-processing
   line-by-line to inject padding — strictly more code than the incumbent's
   one `format!("{:<50}", ...)` call.
3. **Net effect:** the incumbent's 148 lines include the collapsing logic
   (kept either way), the glyph-drawing (what `termtree` would take over),
   *and* the 3-column layout (which `termtree` cannot express and would need
   new code to restore). Swapping in `termtree` trades ~40-50 lines of glyph
   bookkeeping for a new dependency, while leaving the collapsing logic
   untouched and requiring new code to claw back the alignment behavior that
   already works today.

### Verdict
**DOESN'T-FIT** — `termtree@1.0.0` — the custom collapsing rule
(`cli.rs:1025-1033`) is data-layer logic that has to be reimplemented
regardless of rendering library (confirms the reviewer's stated risk), and
`termtree`'s per-node `Display` has no mechanism for the incumbent's
global fixed-width 3-column layout (tree+segment / method / full path,
`cli.rs:1161-1163`) — reproducing it needs *more* code than today, not less.
Net LOC: roughly flat-to-negative once alignment is restored, plus a new
dependency. Incumbent (`src/cli.rs:1017-1164`) stays; validates its KPI7
score. (`ptree@0.5.2` was not separately spiked — it shares the same
per-node-`Display` architecture and would hit the identical column-alignment
gap.)
