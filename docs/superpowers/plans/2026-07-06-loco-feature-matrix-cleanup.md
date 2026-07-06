# Loco Feature-Matrix Cleanup (WS3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Simplify Loco's Cargo feature matrix — drop dead `integration_test`, collapse `bg_*` into `worker`/`worker_redis`, rename `auth_jwt`→`auth`, expose pg/sqlite queues in the wizard, and fix+guard `embedded_assets`.

**Architecture:** Mechanical cfg renames over the lib (the runtime queue dispatch is unchanged), plus `loco-new` wizard/settings/template changes to emit the new features + a backend-selectable `queue:` config. The already-green `loco-new` wizard matrix (`tests/wizard/new.rs`) is the integration harness.

**Tech Stack:** Rust, Cargo features, `loco-new` (Tera `.t` templates + `setup.rhai`), rstest wizard matrix, insta snapshots.

## Global Constraints

- Breaking changes are acceptable (0.17.0 is already breaking via Sea-ORM 2.0). Every breaking feature change gets a CHANGELOG + migration-guide entry.
- Local commits only; never push/PR/publish. Commit trailer required: `Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS`.
- Target default feature set (verbatim): `default = ["auth", "cli", "with-db", "cache_inmem", "worker"]`.
- Feature defs (verbatim): `auth = ["dep:jsonwebtoken", "jsonwebtoken/rust_crypto"]`; `worker = ["dep:sqlx", "dep:ulid"]`; `worker_redis = ["worker", "dep:redis"]`.
- `BackgroundOption` → (feature, `workers.mode`, `queue.kind`) mapping: Async→(none, `BackgroundAsync`, —); QueueRedis→(`worker_redis`, `BackgroundQueue`, `Redis`); QueuePostgres→(`worker`, `BackgroundQueue`, `Postgres`); QueueSqlite→(`worker`, `BackgroundQueue`, `Sqlite`); Blocking→(none, `ForegroundBlocking`, —).
- Verify each lib task with the exact wizard-clippy invocation used by the matrix: `RUSTFLAGS="-D warnings" cargo clippy --manifest-path examples/reference_spa/Cargo.toml -- -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms -A clippy::result_large_err`.
- Run the wizard matrix from `loco-new/` with `LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco cargo test --test mod "wizard::new::test_starter_combinations::case_N" -- --test-threads=1`. NOTE: each run builds a multi-GB temp dir; check `df -h /System/Volumes/Data` before running and remove leaked `$TMPDIR/<5char>` app dirs by explicit name if disk is low.

## File Structure

- `Cargo.toml` — feature definitions + `default` (Tasks 1–3).
- `src/**` — `#[cfg(feature = ...)]` sites: `bg_*` (7 files, 41 occ) and `auth_jwt` (5 files, 6 occ).
- `loco-new/src/wizard.rs` — `BackgroundOption` enum (Task 4); `AssetsOption` + embedded prompt (Task 5).
- `loco-new/src/settings.rs` — feature-list emission + `Background` mapping (Tasks 4–5).
- `loco-new/base_template/config/{development,test}.yaml.t` — `workers.mode` + `queue:` block (Task 4).
- `loco-new/tests/wizard/new.rs` — matrix cases (Task 6).
- `CHANGELOG.md`, `docs-site/content/docs/extras/upgrades.md` — migration notes (folded into each breaking task; consolidated in Task 6).

---

### Task 1: Drop the dead `integration_test` feature

**Files:**
- Modify: `Cargo.toml` (feature line `integration_test = []`)

**Interfaces:**
- Produces: nothing (pure removal).

- [ ] **Step 1: Prove it is dead**

Run: `grep -rn 'integration_test' src/ loco-gen/src/ loco-new/ tests/ examples/ --include='*.rs' --include='*.t'`
Expected: no matches (only the `Cargo.toml` definition exists).

- [ ] **Step 2: Remove the feature**

In `Cargo.toml`, delete the two lines:
```toml
## Testing feature flags
integration_test = []
```

- [ ] **Step 3: Verify the workspace still builds**

Run: `cargo check --features testing`
Expected: `Finished` (no `unknown feature` error anywhere).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore(features)!: remove dead integration_test feature

Zero .rs references anywhere (verified). Part of WS3 feature-matrix cleanup.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

### Task 2: Rename `auth_jwt` → `auth`

**Files:**
- Modify: `Cargo.toml` (feature def + `default`)
- Modify: `src/auth/mod.rs`, `src/controller/extractor/mod.rs`, `src/controller/mod.rs`, `src/model/mod.rs`, `src/prelude.rs` (6 `auth_jwt` occurrences)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: feature `auth` (replaces `auth_jwt`), gating JWT + `ApiToken` exactly as before.

- [ ] **Step 1: Rename the feature definition**

In `Cargo.toml`, change:
```toml
auth_jwt = ["dep:jsonwebtoken", "jsonwebtoken/rust_crypto"]
```
to:
```toml
auth = ["dep:jsonwebtoken", "jsonwebtoken/rust_crypto"]
```
and in `default`, change `"auth_jwt"` → `"auth"`.

- [ ] **Step 2: Sweep the cfg sites**

Run: `grep -rl 'auth_jwt' src/ | xargs perl -i -pe 's/feature = "auth_jwt"/feature = "auth"/g'`
Then: `grep -rn 'auth_jwt' src/`
Expected: no matches remaining.

- [ ] **Step 3: Verify build with and without the feature**

Run: `cargo check` (default has `auth`) then `cargo check --no-default-features --features "cli,with-db,worker,testing"`
Expected: both `Finished` — no `auth`/`auth_jwt` reference errors. (The second confirms `auth`-off still compiles.)

- [ ] **Step 4: Add CHANGELOG breaking entry**

In `CHANGELOG.md`, under the Breaking section, add:
```markdown
- **`auth_jwt` feature renamed to `auth`.** Update `features = ["auth_jwt"]` → `["auth"]`
  (it gates JWT auth and the `ApiToken` extractor, as before).
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/ CHANGELOG.md
git commit -m "refactor(features)!: rename auth_jwt -> auth

Honest name: the feature gates JWT auth and the non-JWT ApiToken extractor.
Pure rename, no split. Part of WS3.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

### Task 3: Collapse `bg_redis`/`bg_pg`/`bg_sqlt` → `worker`/`worker_redis`

**Files:**
- Modify: `Cargo.toml` (feature defs + `default`)
- Modify: `src/bgworker/mod.rs`, `src/cli.rs`, `src/controller/mod.rs`, `src/controller/monitoring.rs`, `src/errors.rs`, `src/tests_cfg/mod.rs`, `src/tests_cfg/queue.rs` (7 files, 41 `bg_*` occurrences)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: features `worker` (Postgres+SQLite backends) and `worker_redis` (`worker` + Redis). The runtime `QueueConfig::{Postgres,Sqlite,Redis}` dispatch in `src/bgworker/mod.rs:756` is unchanged.

- [ ] **Step 1: Replace the feature definitions**

In `Cargo.toml`, replace:
```toml
bg_redis = ["dep:redis", "dep:ulid"]
bg_pg = ["dep:sqlx", "dep:ulid"]
bg_sqlt = ["dep:sqlx", "dep:ulid"]
```
with:
```toml
worker = ["dep:sqlx", "dep:ulid"]
worker_redis = ["worker", "dep:redis"]
```
and in `default`, replace the `"bg_redis", "bg_pg", "bg_sqlt"` entries with a single `"worker"`.

- [ ] **Step 2: Sweep the cfg sites (Redis first, then pg/sqlite → worker)**

Run each, in order:
```bash
grep -rl 'bg_redis\|bg_pg\|bg_sqlt' src/ | xargs perl -i -pe '
  s/feature = "bg_redis"/feature = "worker_redis"/g;
  s/feature = "bg_pg"/feature = "worker"/g;
  s/feature = "bg_sqlt"/feature = "worker"/g;
'
```
This leaves two mechanical artifacts to fix by hand in the next step:
`any(feature = "bg_pg", feature = "bg_sqlt", feature = "bg_redis")` groups become
`any(feature = "worker", feature = "worker", feature = "worker_redis")`, and
`any(feature = "bg_pg", feature = "bg_sqlt")` become `any(feature = "worker", feature = "worker")`.

- [ ] **Step 3: Collapse the now-duplicate `any(...)` groups**

Run: `grep -rn 'feature = "worker", feature = "worker"' src/`
For each hit, replace the redundant `any(feature = "worker", feature = "worker"[, feature = "worker_redis"])` with just `feature = "worker"` (since `worker_redis` implies `worker`, the whole group reduces to `feature = "worker"`). Example — `src/bgworker/mod.rs:249`:
```rust
// before (post-sweep):
#[cfg(any(feature = "worker", feature = "worker", feature = "worker_redis"))]
// after:
#[cfg(feature = "worker")]
```
Then verify none remain: `grep -rn 'feature = "worker", feature = "worker"' src/` → no matches.

- [ ] **Step 4: Verify no `bg_*` remains and the module gating is correct**

Run: `grep -rn 'bg_redis\|bg_pg\|bg_sqlt' src/`
Expected: no matches.
Confirm `src/bgworker/mod.rs` module decls read: `pg`/`sql` mods under `#[cfg(feature = "worker")]`, `redis` mod under `#[cfg(feature = "worker_redis")]`.

- [ ] **Step 5: Verify all three feature combos compile**

Run each:
```bash
cargo check --no-default-features --features "cli,with-db,worker,testing"
cargo check --no-default-features --features "cli,with-db,worker_redis,testing"
cargo check --no-default-features --features "cli,with-db,testing"   # queue disabled
```
Expected: all `Finished`. The third exercises the no-op queue provider path with no `worker` feature.

- [ ] **Step 6: Run the bgworker unit tests**

Run: `cargo test --features "worker_redis,with-db,testing" --lib bgworker 2>&1 | grep 'test result'`
Expected: `ok` (Postgres tests may need Docker; SQLite ones must pass).

- [ ] **Step 7: Add CHANGELOG breaking entry**

```markdown
- **Background-queue features collapsed.** `bg_pg`/`bg_sqlt` → `worker`
  (Postgres+SQLite; free once `sqlx` is compiled), `bg_redis` → `worker_redis`
  (adds `dep:redis`). `default` now has `worker` (not Redis). A Redis queue needs
  `worker_redis`; the queue backend is selected at runtime by `queue.kind`.
```

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/ CHANGELOG.md
git commit -m "refactor(features)!: collapse bg_* into worker/worker_redis

worker = pg+sqlite backends (sqlx already compiled by with-db); worker_redis
adds the Redis backend + crate. Runtime queue dispatch unchanged. Part of WS3.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

### Task 4: Reshape `BackgroundOption` + emit backend-selected queue config

**Files:**
- Modify: `loco-new/src/wizard.rs` (`BackgroundOption` enum + `user_message`/`prompt_view`)
- Modify: `loco-new/src/settings.rs` (feature emission + `Background` mapping)
- Modify: `loco-new/base_template/config/development.yaml.t`, `loco-new/base_template/config/test.yaml.t`

**Interfaces:**
- Consumes: features `worker`/`worker_redis` (Task 3).
- Produces: `BackgroundOption::{Async, QueueRedis, QueuePostgres, QueueSqlite, Blocking}`; `settings.background.kind` (mode string) and `settings.background.queue_kind` (`Redis`/`Postgres`/`Sqlite`/empty).

- [ ] **Step 1: Replace the enum variants** in `loco-new/src/wizard.rs`:
```rust
pub enum BackgroundOption {
    #[default]
    #[strum(to_string = "Async (in-process tokio async tasks)")]
    #[serde(rename = "BackgroundAsync")]
    Async,
    #[strum(to_string = "Queue: Redis (standalone workers)")]
    #[serde(rename = "QueueRedis")]
    QueueRedis,
    #[strum(to_string = "Queue: Postgres (standalone workers)")]
    #[serde(rename = "QueuePostgres")]
    QueuePostgres,
    #[strum(to_string = "Queue: SQLite (standalone workers)")]
    #[serde(rename = "QueueSqlite")]
    QueueSqlite,
    #[strum(to_string = "Blocking (run tasks in foreground)")]
    #[serde(rename = "ForegroundBlocking")]
    Blocking,
}
```

- [ ] **Step 2: Update `user_message`/`prompt_view`** in the same `impl` to cover the new variants:
```rust
    pub fn user_message(&self) -> Option<String> {
        match self {
            Self::QueueRedis | Self::QueuePostgres | Self::QueueSqlite => Some(format!(
                "{}: You've selected `{}` for your background worker configuration \
                 (ensure the selected queue backend is reachable)",
                "workers".underline(), "queue".yellow())),
            Self::Blocking => Some(format!(
                "{}: You've selected `{}` — workers BLOCK REQUESTS until a task is done.",
                "workers".underline(), "blocking".yellow())),
            Self::Async => None,
        }
    }
    pub const fn prompt_view(&self) -> &str {
        match self {
            Self::Async => "Async",
            Self::QueueRedis => "QueueRedis",
            Self::QueuePostgres => "QueuePostgres",
            Self::QueueSqlite => "QueueSqlite",
            Self::Blocking => "ForegroundBlocking",
        }
    }
```

- [ ] **Step 3: Add `queue_kind` + mode mapping** in `loco-new/src/settings.rs`. Find the `Background` settings struct and its `From<BackgroundOption>`; set `kind` (mode) and a new `queue_kind: String`:
```rust
// kind (workers.mode): Async->"BackgroundAsync"; QueueRedis|QueuePostgres|QueueSqlite->"BackgroundQueue"; Blocking->"ForegroundBlocking"
// queue_kind: QueueRedis->"Redis"; QueuePostgres->"Postgres"; QueueSqlite->"Sqlite"; else->"" (empty)
```

- [ ] **Step 4: Update feature emission** in `settings.rs:from_wizard` so the Queue* selections push the right feature (works for both db and non-db apps):
```rust
match prompt_selection.background {
    wizard::BackgroundOption::QueueRedis => features.names.push("worker_redis".to_string()),
    wizard::BackgroundOption::QueuePostgres | wizard::BackgroundOption::QueueSqlite =>
        features.names.push("worker".to_string()),
    _ => {}
}
```
(For db apps that use `Features::default()`, `worker` is already present; still push `worker_redis` for QueueRedis. Ensure the push path runs for both branches — refactor so the match runs after the db/non-db `features` is chosen.)

- [ ] **Step 5: Render the queue block from the backend** in both `config/development.yaml.t` and `config/test.yaml.t`, replacing the hard-coded `kind: Redis` block:
```jinja
  {% if settings.background.kind == "BackgroundQueue" %}
# Queue Configuration
queue:
  kind: {{settings.background.queue_kind}}
  {% if settings.background.queue_kind == "Redis" %}
  uri: {% raw %}{{{% endraw %} get_env(name="REDIS_URL", default="redis://127.0.0.1") {% raw %}}}{% endraw %}
  dangerously_flush: false
  {% elif settings.background.queue_kind == "Postgres" %}
  uri: {% raw %}{{{% endraw %} get_env(name="DATABASE_URL", default="postgres://loco:loco@localhost:5432/loco_development") {% raw %}}}{% endraw %}
  dangerously_flush: false
  {% elif settings.background.queue_kind == "Sqlite" %}
  uri: {% raw %}{{{% endraw %} get_env(name="QUEUE_URL", default="sqlite://loco_development.sqlite?mode=rwc") {% raw %}}}{% endraw %}
  dangerously_flush: false
  {% endif %}
  {% endif %}
```

- [ ] **Step 6: Verify loco-new compiles + wizard tests compile**

Run: `cargo test --manifest-path loco-new/Cargo.toml --test mod --no-run 2>&1 | grep -E 'error|Finished'`
Expected: `Finished` (enum change ripples through `wizard.rs`/`settings.rs` cleanly).

- [ ] **Step 7: Commit**

```bash
git add loco-new/src/wizard.rs loco-new/src/settings.rs loco-new/base_template/config/
git commit -m "feat(loco-new): expose pg/sqlite queue backends in the wizard

BackgroundOption -> Async|QueueRedis|QueuePostgres|QueueSqlite|Blocking, mapped
to worker/worker_redis features + a backend-selected queue: config block. Part of WS3.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

### Task 5: `embedded_assets` wizard toggle + clientside guard

**Files:**
- Modify: `loco-new/src/wizard.rs` (add an `embedded` prompt/field for Serverside)
- Modify: `loco-new/src/settings.rs` (push `embedded_assets` feature; guard)

**Interfaces:**
- Consumes: `AssetsOption::Serverside`.
- Produces: a `settings.embedded_assets: bool` that pushes the `embedded_assets` feature; a hard error when combined with Clientside.

- [ ] **Step 1: Add the guard test** in `loco-new/tests/wizard/new.rs`:
```rust
#[test]
fn embedded_assets_with_clientside_is_rejected() {
    let sel = wizard::Selections {
        db: DBOption::None,
        background: BackgroundOption::Async,
        asset: AssetsOption::Clientside,
    };
    // embedded requested with clientside must be an error
    assert!(settings::Settings::from_wizard_checked("x", &sel, OS::default(), /*embedded=*/ true).is_err());
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test --manifest-path loco-new/Cargo.toml --test mod embedded_assets_with_clientside_is_rejected 2>&1 | grep -E 'error\[|cannot find'`
Expected: FAIL — `from_wizard_checked` does not exist yet.

- [ ] **Step 3: Implement the toggle + guard** in `settings.rs`. Add `embedded_assets: bool` to `Settings`; add a checked constructor that errors on `embedded && asset.is_client_side()`, else pushes the feature:
```rust
pub fn from_wizard_checked(name: &str, sel: &wizard::Selections, os: OS, embedded: bool) -> crate::Result<Self> {
    if embedded && sel.asset == AssetsOption::Clientside {
        return Err(crate::Error::msg("embedded_assets cannot be combined with a clientside app (no assets/ dir to embed)"));
    }
    let mut s = Self::from_wizard(name, sel, os);
    if embedded { s.features.names.push("embedded_assets".to_string()); s.embedded_assets = true; }
    Ok(s)
}
```

- [ ] **Step 4: Add the wizard prompt** in `wizard.rs` — only ask "Embed static assets into the binary?" when `asset == Serverside`; pass the answer to `from_wizard_checked`.

- [ ] **Step 5: Run the guard test — expect PASS**

Run: `cargo test --manifest-path loco-new/Cargo.toml --test mod embedded_assets_with_clientside_is_rejected 2>&1 | grep 'test result'`
Expected: `ok. 1 passed`.

- [ ] **Step 6: Commit**

```bash
git add loco-new/src/wizard.rs loco-new/src/settings.rs loco-new/tests/wizard/new.rs
git commit -m "feat(loco-new): wire embedded_assets (serverside) + guard clientside

Serverside-only wizard toggle pushes the embedded_assets feature; embedded_assets
+ clientside is a hard error (was the silent-404 combo). Part of WS3.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

### Task 6: Extend the wizard matrix + migration guide

**Files:**
- Modify: `loco-new/tests/wizard/new.rs` (matrix cases)
- Modify: `docs-site/content/docs/extras/upgrades.md`
- Modify: any loco-gen/loco-rs snapshots touched by the cfg renames

**Interfaces:**
- Consumes: everything from Tasks 1–5.

- [ ] **Step 1: Add matrix cases** in `loco-new/tests/wizard/new.rs` — a `QueueSqlite` case (exercises `worker` end-to-end without needing Redis/Docker) and a Serverside+embedded case:
```rust
// full-stack SPA with a SQLite queue backend (-> worker feature)
#[case(DBOption::Sqlite, AssetsOption::Clientside, BackgroundOption::QueueSqlite)]
// serverside with embedded assets
#[case(DBOption::None, AssetsOption::Serverside, BackgroundOption::Async)]
```
(Adjust the `#[case]` signature/arity to include `BackgroundOption`; thread it through `test_combination`.)

- [ ] **Step 2: Run the new SQLite-queue combo end-to-end**

Run (from `loco-new/`): `LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco cargo test --test mod "wizard::new::test_starter_combinations" -- --test-threads=1 2>&1 | grep 'test result'`
Expected: `ok` — generated apps with `worker` compile, clippy-clean, and test-pass.
(Check `df -h /System/Volumes/Data` first; clean leaked `$TMPDIR/<5char>` app dirs by explicit name if low.)

- [ ] **Step 3: Refresh any drifted snapshots**

Run: `cargo test -p loco-gen --test mod --features with-db 2>&1 | grep 'test result'`
If a snapshot failed on a cfg rename, review the `.snap` vs `.snap.new` diff (must be only `bg_*`→`worker*` / `auth_jwt`→`auth` changes), then `INSTA_UPDATE=always` re-run and `git diff` to confirm.

- [ ] **Step 4: Write the migration-guide section** in `docs-site/content/docs/extras/upgrades.md`:
```markdown
### Feature-flag changes (0.17)

- `auth_jwt` → `auth`.
- `bg_redis` → `worker_redis`; `bg_pg`/`bg_sqlt` → `worker`. `default` now includes
  `worker` (Postgres+SQLite queues); add `worker_redis` for a Redis queue.
- `integration_test` removed (was dead).
- `loco new` now offers Redis/Postgres/SQLite queue backends and (serverside)
  embedded assets.
```

- [ ] **Step 5: Full workspace green check**

Run: `cargo clippy --features testing 2>&1 | grep -E 'error|warning: ' | head` (expect empty) and `cargo test -p loco-gen --lib --features with-db 2>&1 | grep 'test result'` (expect `ok`).

- [ ] **Step 6: Commit**

```bash
git add loco-new/tests/ docs-site/ loco-gen/tests/
git commit -m "test(loco-new)+docs: WS3 matrix cases + feature migration guide

Adds QueueSqlite (worker) + serverside-embedded matrix combos; upgrade-guide
feature-flag section; refreshes cfg-rename snapshots. Closes WS3.

Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS"
```

---

## Self-Review

**Spec coverage:** ① drop `integration_test` → Task 1 ✓. ② `auth` rename → Task 2 ✓. ③ `bg_*`→`worker`/`worker_redis` → Task 3 ✓. ④ wizard pg/sqlite queues → Task 4 ✓. ⑤ `embedded_assets` fix+guard → Task 5 ✓. Testing strategy (matrix cases, feature-combo checks, snapshots, migration notes) → Tasks 3/6 ✓. Default feature set applied in Tasks 2+3 ✓.

**Type consistency:** `BackgroundOption` variants `{Async, QueueRedis, QueuePostgres, QueueSqlite, Blocking}` used identically in Tasks 4 and 6. `settings.background.queue_kind` (`Redis`/`Postgres`/`Sqlite`/empty) defined in Task 4 Step 3, consumed by the config template Task 4 Step 5. `from_wizard_checked(name, sel, os, embedded) -> Result<Settings>` defined and used consistently in Task 5. Features `worker`/`worker_redis`/`auth` named identically across Tasks 2–4.

**Known adaptation points (not placeholders — flagged for the implementer):** the exact `Background` settings struct + its `From<BackgroundOption>` (Task 4 Step 3) and the wizard prompt plumbing (Task 5 Step 4) must be located in `settings.rs`/`wizard.rs`; the surrounding code is shown, the struct field names are to be matched to the existing struct. The rstest `#[case]` arity change (Task 6 Step 1) must thread `BackgroundOption` through `test_combination`, which currently takes `(db, asset)`.
