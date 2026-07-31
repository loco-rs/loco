+++
title = "CLI reference"
description = "Every flag and subcommand for the `loco` app generator and the `cargo loco` runtime CLI."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/getting-started/starters/"]

[extra]
lead = ""
toc = true
top = false
+++

Loco ships **two** command-line surfaces:

| Binary | Crate | Installed via | Purpose |
|---|---|---|---|
| `loco` | `loco-new` (binary crate, not `loco-rs`) | `cargo install loco` | Scaffolds a new app on disk. One subcommand: `new`. |
| `cargo loco` | generated into every app by `loco new`, backed by `loco_rs::cli` | built with your app | Runtime operations against *your* app: start the server, run migrations, generate code, run tasks, etc. |

`cargo loco` has **two `main()` implementations**, selected by the `with-db` Cargo feature (`src/cli.rs:712` when `with-db` is on, `src/cli.rs:872` when it is off). The non-`with-db` build has no `Db` subcommand and no DB-backed generators (`model`/`migration`/`scaffold`). Both variants share the same top-level `-e, --environment <ENV>` global flag (default `development`).

---

## 1. `loco new` — the app generator

Source: `loco-new/src/bin/main.rs:30-62`.

### 1.1 Global flag

| Flag | Type | Default | Purpose |
|---|---|---|---|
| `-l, --log <LEVEL>` | `LevelFilter` | `ERROR` | Verbosity of the generator's own logging (`main.rs:22-24`) |

### 1.2 `new` flags

| Flag | Type | Default | Purpose |
|---|---|---|---|
| `-p, --path <PATH>` | `PathBuf` | `.` | Local directory to generate into (`main.rs:34-36`) |
| `-n, --name <NAME>` | `Option<String>` | none — prompts | App name (`main.rs:38-40`) |
| `--db <DB>` | `wizard::DBOption` | none — prompts | DB provider: `sqlite` \| `postgres` \| `none` (`main.rs:42-44`) |
| `--bg <BG>` | `wizard::BackgroundOption` | none — prompts | Background-worker mode: `async` \| `queue-redis` \| `queue-postgres` \| `queue-sqlite` \| `blocking` (`main.rs:46-48`) |
| `--assets <ASSETS>` | `wizard::AssetsOption` | none — prompts | Asset serving: `serverside` \| `clientside` \| `none` (`main.rs:50-52`) |
| `--embedded-assets` | `bool` | `false` — prompts (interactive, serverside only) | Embed static assets into the binary (`embedded_assets` feature). Serverside only; combined with `--assets clientside` it is a hard error. Supplying it (or running fully flag-driven) skips the interactive prompt |
| `-a, --allow-in-git-repo` | `bool` | `false` | Skip the "you're inside a git repo, continue?" abort prompt (`main.rs:54-56`) |
| `--os <OS>` | `wizard::OS` | `linux` on Unix, `windows` otherwise | Generate a Unix- or Windows-optimized starter: `windows` \| `linux` \| `macos` (`main.rs:58-60`, `DEFAULT_OS` `main.rs:64-67`) |

There is **no `-t/--template`** and **no `-v/--verbose`** flag. The `Template` enum exists internally and derives `ValueEnum`, but it is not wired to any CLI argument — template choice is interactive-only (§1.4).

If `--db`, `--bg`, **and** `--assets` are all supplied, the wizard skips every prompt except app name (`wizard.rs:287-297`); app name still prompts unless `--name` is also given.

### 1.3 Interactive prompts (when flags are omitted)

Source: `loco-new/src/wizard.rs`.

1. **App name?** (default `myapp`) — non-empty, no leading digit, Unicode XID + `-`/`_` (`wizard.rs:201-267`).
2. **"You are inside a git repository. Do you wish to continue?"** — only if `cwd` is a git repo and `--allow-in-git-repo` was not passed; default No, aborts on decline (`wizard.rs:227-240`; `main.rs:91-94`).
3. **"What would you like to build?"** — template select (`wizard.rs:299-302`).
4. Conditional DB / background / assets follow-ups, depending on the chosen template.

### 1.4 Templates

Source: `wizard.rs:12-27`, branch logic `wizard.rs:304-333`.

| Template (enum) | Menu label | DB prompt? | BG prompt? | Assets |
|---|---|---|---|---|
| `SaasServerSideRendering` (default) | "Saas App with server side rendering" | yes | yes | forced `Serverside` |
| `SaasClientSideRendering` | "Saas App with client side rendering" | yes | yes | forced `Clientside` |
| `RestApi` | "Rest API (with DB and user auth)" | yes | yes | forced `None` |
| `Lightweight` | "lightweight-service (minimal, only controllers and views)" | no — forced `None` | no — forced `Async` | forced `None` |
| `Advanced` | "Advanced" | yes | yes | **asks** (only template that prompts for asset config) |

### 1.5 Option enums (clap values + menu labels)

**`DBOption`** — `wizard.rs:39-79` — clap `--db` values: `sqlite` (default), `postgres`, `none`.
| Value | Endpoint template | Notes |
|---|---|---|
| `sqlite` | `sqlite://NAME_ENV.sqlite?mode=rwc` | default |
| `postgres` | `postgres://loco:loco@localhost:5432/NAME_ENV` | warns a running Postgres instance is required |
| `none` | — | `enable()` is `false`; disables DB, auth, and mailer generation |

**`BackgroundOption`** — `wizard.rs:81-125` — clap `--bg` values: `async` (default), `queue-redis`, `queue-postgres`, `queue-sqlite`, `blocking`.
| Value | Menu label | Notes |
|---|---|---|
| `async` | "Async (in-process tokio async tasks)" | default |
| `queue-redis` | "Queue: Redis (standalone workers)" | warns the selected queue backend must be reachable; generates with the `worker_redis` feature |
| `queue-postgres` | "Queue: Postgres (standalone workers)" | warns the selected queue backend must be reachable; generates with the `worker` feature |
| `queue-sqlite` | "Queue: SQLite (standalone workers)" | warns the selected queue backend must be reachable; generates with the `worker` feature |
| `blocking` | "Blocking (run tasks in foreground)" | warns it **blocks requests** until the task completes |

**`AssetsOption`** — `wizard.rs:127-162` — clap `--assets` values: `serverside` (default), `clientside`, `none`.
| Value | Menu label | Notes |
|---|---|---|
| `serverside` | "Server (configures server-rendered views)" | default |
| `clientside` | "Client (configures assets for frontend serving)" | prints follow-up: `cd frontend/ && npm install && npm run build` |
| `none` | "None" | — |

**`OS`** — `loco-new/src/lib.rs:41-51` — clap `--os` values: `windows`, `linux`, `macos`. `windows` adds a second `tool` bin target to the generated `Cargo.toml` (`Cargo.toml.t:65-70`).

### 1.6 Derived generation settings

Source: `loco-new/src/settings.rs:58-89`.

- DB enabled → `Features::default()` (loco-rs default features apply to the generated app).
- DB **disabled** (Lightweight template, or `--db none`) → `default-features = false`, feature names = `["cli"]`; if background is `queue-redis`, `"worker_redis"` is appended; if background is `queue-postgres` or `queue-sqlite`, `"worker"` is appended.
- `auth` and `mailer` scaffolding are enabled iff DB is enabled.
- Serverside assets → generated `Initializers { view_engine: true }`.
- `loco_version_text`: normally `version = "0.17"`; when env var `LOCO_DEV_MODE_PATH` is set, becomes `version = "*", path = "<that path>"` — this is how the local framework checkout is dogfooded.
- Generated app's own edition is pinned in `loco-new/base_template/Cargo.toml.t` independent of the `loco-rs` framework edition.

---

## 2. `cargo loco` — the runtime CLI

Source: `src/cli.rs:64-171` (`enum Commands`).

### 2.1 Top-level subcommands

| Subcommand | Alias | Gated on | Flags | Purpose |
|---|---|---|---|---|
| `start` | `s` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `-a/--all`, `--scheduler`, `-b/--binding <ADDR>`, `-p/--port <PORT>`, `-n/--no-banner` (`worker`/`server_and_worker`/`all` are mutually exclusive) | Boot the app in a start mode (`cli.rs:66-91`) |
| `db` | — | `#[cfg(feature = "with-db")]` | see §2.2 | Database operations (`cli.rs:92-97`) |
| `routes` | — | — | none | Print all application endpoints as a tree (`cli.rs:98-99`) |
| `middleware` | — | — | `-c/--config` | List middlewares (enabled first, then disabled); `--config` also prints each one's resolved config (`cli.rs:101-105`) |
| `task` | `t` | — | `[name]`, `key:val...` params | Run a custom task by name, with `key:value` params (`cli.rs:107-114`) |
| `jobs` | — | `#[cfg(feature = "worker")]` | see §2.3 | Manage the background jobs queue (`cli.rs:115-120`) |
| `scheduler` | — | — | `-n/--name <NAME>`, `-t/--tag <TAG>`, `-c/--config <PATH>`, `-l/--list` | Run or inspect the scheduler (`cli.rs:121-137`) |
| `generate` | `g` | `#[cfg(debug_assertions)]` | see §2.4 | Code generation (`cli.rs:138-146`) |
| `doctor` | — | — | `-c/--config`, `-p/--production` | Validate/diagnose the app; `--config` instead dumps the resolved config + environment and skips checks (`cli.rs:147-154`, `814-832`) |
| `version` | — | — | none | Print the app version (`cli.rs:155-156`) |
| `watch` | `w` | — | `-w/--worker[=tags]`, `-s/--server-and-worker`, `--scheduler` | Wraps `cargo-watch -s 'cargo loco start ...'` (`cli.rs:158-170`, `838-867`); requires `cargo-watch` installed |

**Start-mode resolution** (`cli.rs:736-750`): `--all` or (`--server-and-worker` **and** `--scheduler`) → `All`; `--server-and-worker` alone → `ServerAndWorker`; `--worker[=tags]` (+ `--scheduler`) → `WorkerAndScheduler` / `WorkerOnly`; `--scheduler` alone → `ServerAndScheduler`; none of the above → `ServerOnly`.

### 2.2 `db` subcommands

`enum DbCommands`, `src/cli.rs:483-523` — only present when `with-db` is enabled.

| Command | Flags | Purpose |
|---|---|---|
| `create` | — | Create the schema/database |
| `migrate` | — | Apply all pending up-migrations |
| `down` | `[steps]` (default `1`) | Roll back the given number of migrations |
| `reset` | — | Drop all tables, then reapply every migration |
| `status` | — | Show migration status |
| `entities` | — (`#[cfg(debug_assertions)]`) | Generate entity `.rs` files from the current DB schema |
| `truncate` | — | Truncate table data without dropping tables |
| `seed` | `-r/--reset`, `-d/--dump`, `--dump-tables <csv>`, `--from <DIR>` (default `src/fixtures`) | Seed the DB from files, or dump tables to files (`cli.rs:505-520`) |
| `schema` | — | Dump the database schema |

### 2.3 `jobs` subcommands

`enum JobsCommands`, `src/cli.rs:598-647` — only present when the `worker` feature is enabled (`worker_redis` implies `worker`).

| Command | Flags | Purpose |
|---|---|---|
| `cancel` | `--name <NAME>` (required) | Set jobs matching `<NAME>` to `cancelled` |
| `tidy` | — | Delete jobs that are `completed` or `cancelled` |
| `purge` | `--max-age <DAYS>` (default `90`), `--status <csv>`, `--dump <PATH>` | Delete old failed/cancelled jobs, optionally dumping them first |
| `dump` | `--status <csv>`, `-f/--folder <DIR>` (default `.`) | Save job details to files |
| `import` | `-f/--file <PATH>` | Import jobs from a file |
| `requeue` | `--from-age <MINS>` (default `0`) | Move `processing` jobs older than the given age back to `queued` |

### 2.4 `generate` subcommands

`enum ComponentArg`, `src/cli.rs:173-382` — only present in debug builds (`#[cfg(debug_assertions)]` on `Commands::Generate`). `model`/`migration`/`scaffold` are additionally gated on `#[cfg(feature = "with-db")]`. Full field-type syntax is covered in the [Generators & field types](@/docs/reference/generators.md) reference; this table lists CLI shape only.

| Command | Gated | Args / flags | Notes |
|---|---|---|---|
| `model` | `with-db` | `name`, `--without-tz`, `field:type ...` | Fields support `references` (e.g. `director:references`) |
| `migration` | `with-db` | `name`, `--without-tz`, `field:type ...` | Add/remove columns, join tables, empty migrations, references |
| `scaffold` | `with-db` | `name`, `--without-tz`, `field:type ...` | Adaptive, no kind flag; JSON API controller + typed React hooks/pages when the app has a `frontend/`. `--api`/`--html`/`--htmx` accepted for compat |
| `controller` | — | `name`, `actions...` | JSON API controller, no kind flag; `--api`/`--html`/`--htmx` accepted for compat |
| `task` | — | `name` | |
| `scheduler` | — | none | Scaffolds a scheduler config template |
| `worker` | — | `name` | |
| `mailer` | — | `name` | |
| `data` | — | `name` | Data loader |
| `deployment` | — | `kind` = `docker` \| `nginx` (`DeploymentKind`, `cli.rs:554-558`) | `docker` inspects static-assets config + `frontend/package.json`; `nginx` uses configured host/port |
| `override` | — | `[template_path]`, `--info` | Copies a built-in generator template locally for customization; no path lists all available templates |

The `-k/--kind` flags and the `ScaffoldKind` enum were **removed** in 1.0. For back-compat, `scaffold` and `controller` still accept `--api` (a no-op) and `--html`/`--htmx` (which error with a pointer to the React SPA frontend), so pre-1.0 commands don't break.

---

## 3. `cargo loco --help` (verified output)

The top-level help, matching `enum Commands` exactly:

```sh
The one-person framework for Rust

Usage: demo_app-cli [OPTIONS] <COMMAND>

Commands:
  start       Start an app
  db          Perform DB operations
  routes      Describe all application endpoints
  middleware  Describe all application middlewares
  task        Run a custom task
  jobs        Managing jobs queue
  scheduler   Run the scheduler
  generate    code generation creates a set of files and code templates based on a predefined set of rules
  doctor      Validate and diagnose configurations
  version     Display the app version
  watch       Watch and restart the app
  help        Print this message or the help of the given subcommand(s)

Options:
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
