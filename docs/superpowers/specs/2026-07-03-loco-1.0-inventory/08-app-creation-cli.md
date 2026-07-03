# Inventory 08 — App Creation Experience, `cargo loco` CLI, Feature-Flag Matrix

Scope: verified against code on branch `release/0.17.0`. Workspace version 0.17.0.
Sources of truth:
- `loco-new/` crate (the `loco new` generator/wizard)
- `src/cli.rs` (the `cargo loco` runtime CLI)
- root `Cargo.toml` `[features]`
- docs under `docs-site/content/docs/`

---

## 1. `loco new` flow (the app generator) — VERIFIED

The generator binary is a **separate crate** `loco` (`loco-new/`), not `loco-rs`. Binary name = `loco`, version 0.17.0, edition 2024 (`loco-new/Cargo.toml`). Installed via `cargo install loco`. It has exactly ONE subcommand: `new`.

### 1.1 `loco new` CLI flags — `loco-new/src/bin/main.rs:30-62`

| Flag | Type | Default | Purpose |
|---|---|---|---|
| `-p, --path <PATH>` | PathBuf | `.` | Local path to generate into (`main.rs:34-36`) |
| `-n, --name <NAME>` | Option<String> | — (prompts) | App name (`:38-40`) |
| `--db <DB>` | `wizard::DBOption` | — (prompts) | DB provider (`:42-44`) |
| `--bg <BG>` | `wizard::BackgroundOption` | — (prompts) | Background worker config (`:46-48`) |
| `--assets <ASSETS>` | `wizard::AssetsOption` | — (prompts) | Assets serving config (`:50-52`) |
| `-a, --allow-in-git-repo` | bool | false | Skip the "inside a git repo" abort prompt (`:54-56`) |
| `--os <OS>` | `wizard::OS` | `linux` on unix / `windows` otherwise | Generate Unix- or Windows-optimized starter (`:58-60`, `DEFAULT_OS :64-67`) |
| `-l, --log <LEVEL>` (global) | LevelFilter | `ERROR` | Verbosity (`main.rs:22-24`) |

NOTE: There is **NO `-t/--template` flag** and **no `-v/--verbose` flag** in the code. Template is chosen interactively only (see 1.3). The `Template` enum derives `ValueEnum` but is not wired to any CLI arg.

If `--db`, `--bg`, AND `--assets` are all supplied, the wizard skips all prompts and template selection entirely (`wizard.rs:288-297`). App name still prompts unless `--name` given.

### 1.2 Wizard prompts (interactive) — `loco-new/src/wizard.rs`

1. `❯ App name?` (default `myapp`) — validated: non-empty, no leading digit, Unicode XID chars + `-`/`_` (`wizard.rs:201-267`).
2. `❯ You are inside a git repository. Do you wish to continue?` — only if cwd is a git repo and `--allow-in-git-repo` not passed; default No, aborts if declined (`wizard.rs:227-240`, `main.rs:91-94`).
3. `❯ What would you like to build?` — the template select (`wizard.rs:299-302`).
4. Conditional follow-ups (DB / background / assets), depending on template.

### 1.3 Templates / starters — `wizard.rs:12-27` + branch logic `wizard.rs:304-333`

FIVE templates (display strings exact from `#[strum(to_string=...)]`):

| Template (enum) | Menu label | DB prompt? | BG prompt? | Assets |
|---|---|---|---|---|
| `SaasServerSideRendering` (default) | "Saas App with server side rendering" | yes | yes | forced `Serverside` |
| `SaasClientSideRendering` | "Saas App with client side rendering" | yes | yes | forced `Clientside` |
| `RestApi` | "Rest API (with DB and user auth)" | yes | yes | `None` |
| `Lightweight` | "lightweight-service (minimal, only controllers and views)" | no (forced `None`) | no (forced `Async`) | `None` |
| `Advanced` | "Advanced" | yes | yes | **asks** (select_asset) |

Only `Advanced` prompts for the asset config; SaaS variants hard-code it; RestApi/Lightweight force None.

### 1.4 Option enums (menu labels + clap values)

**DBOption** (`wizard.rs:39-79`) — clap `--db` values: `sqlite`, `postgres`, `none`. Menu order: Sqlite (default), Postgres, None.
- Sqlite endpoint: `sqlite://NAME_ENV.sqlite?mode=rwc`
- Postgres endpoint: `postgres://loco:loco@localhost:5432/NAME_ENV` + warns you need a running postgres.
- `enable()` = not None (drives auth/mailer/db feature inclusion).

**BackgroundOption** (`wizard.rs:81-125`) — clap `--bg` values: `async`, `queue`, `blocking`. Menu labels:
- Async (default) — "Async (in-process tokio async tasks)"
- Queue — "Queue (standalone workers using Redis)" — warns you need Redis/valkey.
- Blocking — "Blocking (run tasks in foreground)" — warns it BLOCKS requests.

**AssetsOption** (`wizard.rs:127-162`) — clap `--assets` values: `serverside`, `clientside`, `none`. Menu labels: Server (default) / Client / None. Clientside prints "build your frontend: cd frontend/ && npm install && npm run build".

**OS** (`lib.rs:41`) — clap `--os` values: `windows`, `linux`, `macos`. Windows adds a second `tool` bin to generated `Cargo.toml.t:65-70`.

### 1.5 Wizard → Settings → generated features — `loco-new/src/settings.rs:58-89`

- If DB enabled → `Features::default()` (`default_features = true`, no extra names) → generated app uses loco-rs default features.
- If DB **disabled** (Lightweight, or `--db none`) → `Features::disable_features()` = `default_features = false`, names = `["cli"]`; and if background == Queue, appends `"bg_redis"` (`settings.rs:62-70`, `160-169`).
- `auth` and `mailer` are enabled iff DB enabled (`settings.rs:75-76`).
- Serverside assets → `Initializers { view_engine: true }` (`settings.rs:80-84`).
- `loco_version_text`: normally `version = "0.17"` (`LOCO_VERSION` = "0.17", `lib.rs:12`); when `LOCO_DEV_MODE_PATH` env set, becomes `version="*", path="..."` (`settings.rs:110-118`).

### 1.6 Generated app manifest — `loco-new/base_template/Cargo.toml.t`

- **Generated app edition = `2021`** (`Cargo.toml.t:12`) — NOTE for 1.0: the framework itself is edition 2024 / rustc 1.94, but generated apps are still pinned to 2021.
- Generated app version `0.1.0`, `publish = false`, default-run `<module>-cli`.
- Pulls `loco-rs` with `default-features = false` only when DB disabled.
- DB path adds `sea-orm = "2.0.0-rc"` (sqlx-sqlite + sqlx-postgres + runtime-tokio-rustls + macros), migration crate, chrono, validator, uuid.
- Post-generate: runs `cargo fmt` in the new dir (`main.rs:136-143`).

---

## 2. `cargo loco` CLI (runtime) — `src/cli.rs` — COMPLETE

Top-level: `-e, --environment <ENV>` global (default `development`) (`cli.rs:59-62`). Two `main()` variants gated on `with-db` feature (`cli.rs:709` vs `871`) — the non-db build omits `Db` and DB-only generators.

### 2.1 Subcommands (`enum Commands`, `cli.rs:64-171`)

| Subcommand | Alias | Gated | Flags | Purpose |
|---|---|---|---|---|
| `start` | `s` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `-a/--all`, `--scheduler`, `-b/--binding`, `-p/--port`, `-n/--no-banner` (mutually-exclusive group `start_mode`) | Boot app in a start mode (`cli.rs:66-91`, `726-760`) |
| `db` | — | `#[cfg(with-db)]` | subcommands (see 2.2) | DB operations (`cli.rs:92-97`) |
| `routes` | — | — | none | List all endpoints as a tree (`cli.rs:98-99`, printer `1166`) |
| `middleware` | — | — | `-c/--config` | List middlewares (enabled/disabled), optionally with config (`cli.rs:101-105`) |
| `task` | `t` | — | `[name]`, `key:val` params | Run a custom task (`cli.rs:107-114`) |
| `jobs` | — | `#[cfg(bg_redis\|bg_pg\|bg_sqlt)]` | subcommands (see 2.3) | Manage the jobs queue (`cli.rs:115-120`) |
| `scheduler` | — | — | `-n/--name`, `-t/--tag`, `-c/--config <path>`, `-l/--list` | Run/inspect the scheduler (`cli.rs:121-137`) |
| `generate` | `g` | `#[cfg(debug_assertions)]` | subcommands (see 2.4) | Code generation (`cli.rs:138-146`) |
| `doctor` | — | — | `-c/--config`, `-p/--production` | Validate/diagnose config; `--config` dumps config (`cli.rs:147-154`, `814`) |
| `version` | — | — | none | App version (`cli.rs:155-156`) |
| `watch` | `w` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `--scheduler` | Wraps `cargo-watch -s 'cargo loco start ...'` (`cli.rs:158-170`, `838`) — requires `cargo-watch` installed |

`start` mode resolution (`cli.rs:736-750`): `all` or (`server_and_worker`+`scheduler`) → All; `server_and_worker` → ServerAndWorker; `worker` (+scheduler) → WorkerAndScheduler / WorkerOnly; `scheduler` alone → ServerAndScheduler; else ServerOnly.

### 2.2 `db` subcommands (`enum DbCommands`, `cli.rs:483-523`) — only with `with-db`

| Cmd | Flags | Purpose |
|---|---|---|
| `create` | — | Create schema/database (`cli.rs:486`, special-cased `762-767`) |
| `migrate` | — | Apply up migrations |
| `down` | `[steps]` (default 1) | Roll back N migrations |
| `reset` | — | Drop all tables + reapply |
| `status` | — | Migration status |
| `entities` | `#[cfg(debug_assertions)]` | Generate entity `.rs` from schema |
| `truncate` | — | Truncate tables (no drop) |
| `seed` | `-r/--reset`, `-d/--dump`, `--dump-tables <csv>`, `--from <dir>` (default `src/fixtures`) | Seed from / dump to files (`cli.rs:505-520`) |
| `schema` | — | Dump database schema |

### 2.3 `jobs` subcommands (`enum JobsCommands`, `cli.rs:598-647`) — needs a bg_* feature

| Cmd | Flags | Purpose |
|---|---|---|
| `cancel` | `--name <name>` | Set matching jobs to `cancelled` |
| `tidy` | — | Delete completed + cancelled jobs |
| `purge` | `--max-age <days>` (default 90), `--status <csv>`, `--dump <path>` | Delete old failed/cancelled jobs, optional dump first |
| `dump` | `--status <csv>`, `-f/--folder <dir>` (default `.`) | Save jobs to files |
| `import` | `-f/--file <path>` | Import jobs from a file |
| `requeue` | `--from-age <mins>` (default 0) | Move `processing` → `queued` |

### 2.4 `generate` subcommands (`enum ComponentArg`, `cli.rs:173-382`) — debug builds only

| Cmd | Gated | Flags | Notes |
|---|---|---|---|
| `model` | with-db | `--without-tz`, `field:type...` | fields incl. `references` |
| `migration` | with-db | `--without-tz`, `field:type...` | Create/alter/join-table/etc. |
| `scaffold` | with-db | `--without-tz`, fields, `-k/--kind`, `--htmx`/`--html`/`--api` (one required, group `scaffold_kind_group`) | full CRUD |
| `controller` | — | actions, `-k/--kind`, `--htmx`/`--html`/`--api` (one required) | |
| `task` | — | `name` | |
| `scheduler` | — | none | scaffolds scheduler config |
| `worker` | — | `name` | |
| `mailer` | — | `name` | |
| `data` | — | `name` | data loader |
| `deployment` | — | `kind` = `docker` \| `nginx` (`DeploymentKind` `cli.rs:554-558`) | Docker inspects static-assets config + `frontend/package.json`; Nginx uses host/port |
| `override` | — | `[template_path]`, `--info` | Copy/override built-in templates locally; no path → list all |

Kind selection errors out if none of `--kind/--htmx/--html/--api` given (`cli.rs:426-429`, `455-458`).

---

## 3. Feature-flag matrix — root `Cargo.toml:27-64` — COMPLETE

`default = [auth_jwt, cli, with-db, cache_inmem, bg_redis, bg_pg, bg_sqlt]` (`Cargo.toml:28-36`).

| Flag | Default? | Enables (deps / sub-features) | Notes |
|---|---|---|---|
| `auth_jwt` | ON | `dep:jsonwebtoken`, `jsonwebtoken/rust_crypto` | jsonwebtoken 10 no longer bundles crypto; selects pure-Rust backend, self-contained, no C toolchain (`Cargo.toml:37-41`) |
| `cli` | ON | `dep:clap` | Enables `cargo loco` CLI (`:42`) |
| `with-db` | ON | `dep:sea-orm`, `dep:sea-orm-migration`, `dep:sqlx`, `loco-gen/with-db` | Sea-ORM 2.0.0-rc; gates `Db` CLI + model/migration/scaffold gens (`:44-49`) |
| `testing` | OFF | `dep:axum-test`, `dep:scraper`, `dep:tree-fs` | Test harness; also enabled for docs.rs (`:43`, `:211-212`) |
| `cache_inmem` | ON | `dep:moka` | In-memory cache (`:56`) |
| `cache_redis` | OFF | `dep:bb8-redis`, `dep:bb8` | Redis cache pool (`:57`) |
| `bg_redis` | ON | `dep:redis`, `dep:ulid` | Redis-backed queue workers (`:58`) |
| `bg_pg` | ON | `dep:sqlx`, `dep:ulid` | Postgres-backed workers (`:59`) |
| `bg_sqlt` | ON | `dep:sqlx`, `dep:ulid` | SQLite-backed workers (`:60`) |
| `all_storage` | OFF | `storage_aws_s3` + `storage_azure` + `storage_gcp` | Umbrella (`:51`) |
| `storage_aws_s3` | OFF | `opendal/services-s3` | (`:52`) |
| `storage_azure` | OFF | `opendal/services-azblob` | (`:53`) |
| `storage_gcp` | OFF | `opendal/services-gcs` | (`:54`) |
| `embedded_assets` | OFF | (no deps) build-time flag | Embeds `assets/` into the binary; toggles view-engine source (`:64`) |
| `integration_test` | OFF | (empty) | Test-gating marker only (`:62`) |

Any of `bg_redis`/`bg_pg`/`bg_sqlt` unlocks the `jobs` CLI subcommand and `JobStatus` (`cli.rs:27`, `115`, `598`). `debug_assertions` (not a Cargo feature) gates `generate` + `db entities`.

---

## 4. Current doc coverage — ratings + concrete discrepancies

### `getting-started/starters.md` — **STALE**
- **STALE / fabricated `loco new --help` block (lines 52-68):** advertises `-t, --template <TEMPLATE>` and `-v, --verbose <VERBOSE>`. Neither exists in `loco-new/src/bin/main.rs`. Real flags: `-p/--path`, `-n/--name`, `--db`, `--bg`, `--assets`, `-a/--allow-in-git-repo`, `--os`, and global `-l/--log`. The doc is missing `--allow-in-git-repo` and `--os`, and invents `--template`.
- **STALE possible-values:** doc says `--db [sqlite, postgres]`, `--assets [serverside, clientside]`. Actual clap values include `none` for both. `--bg [async, queue, blocking]` is correct.
- **MISSING:** the `Advanced` template is not documented at all (only SaaS / Rest API / Lightweight described). Advanced is the only one that prompts for asset config.
- **THIN:** does not mention `--os` (windows/linux/macos) or the git-repo guard prompt.
- ACCURATE: interactive sample flow (lines 27-44) matches wizard prompt text.

### `getting-started/guide.md` — **ACCURATE (thin)**
- New-app snippet (lines 60-82) matches wizard output. Generator command list driven by snippets. No feature-flag or full-CLI coverage (out of scope for the tutorial). No stale claims found.

### `the-app/your-project.md` — **ACCURATE**
- Lines 49-73: `cargo loco --help` output is an auto-`exec` snippet and matches `Commands` in `cli.rs` exactly (start/db/routes/middleware/task/jobs/scheduler/generate/doctor/version/watch). Good canonical CLI reference. Does not enumerate per-subcommand flags (e.g. `start --all`, `db seed --dump`, `jobs purge`), so those are effectively **MISSING** from prose docs.

### Feature flags — **THIN / no central matrix**
- No single doc enumerates the feature matrix. Flags appear scattered: `embedded_assets` (`the-app/views.md:320-354`), `cache_inmem`/`cache_redis` (`infrastructure/cache.md:51,59`; `the-app/controller.md:317`), `storage_*`/`all_storage` (`infrastructure/storage.md:23-26`), `with-db` (`the-app/controller.md:316`).
- **MISSING from docs entirely:** `auth_jwt`, `cli`, `bg_redis`/`bg_pg`/`bg_sqlt`, `testing`, `integration_test` as documented feature flags with default-on/off status. No mention that `bg_*` gate the `jobs` subcommand, or that `generate`/`db entities` require debug builds.

### `cargo loco` subcommands — **partial**
- `jobs` and its 6 subcommands: no dedicated doc coverage of flags. `doctor`, `middleware`, `watch`, `db seed/schema/truncate` under-documented. Scheduler doc exists (`processing/scheduler.md`).

---

## 5. 1.0-relevant notes

- **Generated-app edition lag:** generated apps are edition **2021** (`base_template/Cargo.toml.t:12`) while the framework is edition 2024 / rust-version **1.94** (root `Cargo.toml:6,11`). Candidate to bump to 2024 for 1.0. Generated app pins `sea-orm = "2.0.0-rc"` and `loco-rs = "0.17"` — both need a stable-1.0 bump.
- **Sea-ORM 2.0 is still `-rc`** (`Cargo.toml:73,201`; template `:35`). Release program notes (git log) gate publish on Sea-ORM 2.0.0 stable.
- **No starter/subcommand removed vs prior:** wizard still 5 templates; `Advanced` present. CLI subcommand set is stable. `data` generator (data loader) and `deployment docker|nginx` are present.
- **Doc snippets use an inject/exec system** (`<!-- <snip ...> -->`); the `your-project.md` help block auto-execs so stays fresh, but `starters.md`'s help block is hand-written and has drifted — for 1.0 it should be converted to an exec snippet or corrected.
