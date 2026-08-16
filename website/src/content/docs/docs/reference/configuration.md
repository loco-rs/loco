---
title: Configuration
description: "Exhaustive reference for every YAML key in a Loco app's config file: loading precedence, environment resolution, and every sub-config struct."
sidebar:
  order: 1
---

This page is a dictionary of every key Loco's configuration loader understands. It documents `struct Config` (`src/config/mod.rs`) and its sub-structs (`src/config/{auth,server,database,logger,mailer,queue,cache}.rs`). For a narrative walkthrough of settings and environments, see the-app/your-project.

## Loading & precedence

- Default config folder: `config/` (`Config::new`, `src/config/mod.rs:128-131`). Override with the `LOCO_CONFIG_FOLDER` env var (read by `Environment::load`, `src/environment.rs:59-64`).
- `Config::from_folder(env, path)` (`src/config/mod.rs:154-200`) loads **both** `{path}/{env}.yaml` and `{path}/{env}.local.yaml` when they exist and **deep-merges** them, with the `.local.yaml` side winning key by key (`merge_yaml`, `src/config/mod.rs:243-257`). The merge recurses through mappings, so a local file only has to restate the keys it overrides; anything that is not a mapping — a scalar or a sequence — is replaced wholesale rather than combined. When only one of the two files exists, that file is used on its own. The loaded path is reported as `"{base} (merged with {local})"` when both were read.

  If neither exists, loading fails with `Error::Message("no configuration file found in folder: ...")`.
- Before parsing, the entire YAML file is rendered as a template (`Config::load_yaml_value`, `src/config/mod.rs:211-217`; `src/config/template.rs`). Config files use the YAML-safe `<%= expr %>` / `<% stmt %>` / `<%# comment %>` delimiters — `<` is not a YAML indicator character, so a templated value like `port: <%= get_env(name="PORT", default="5150") %>` is still valid, unmangled YAML before it's ever rendered. Under the hood these are translated into Tera's native `{{ }}`/`{% %}`/`{# #}` delimiters (`to_tera_syntax`, `src/config/template.rs:95-145`) and then rendered by `tera::render_string` (`src/tera.rs:55-60`), which builds a bare Tera instance and registers Loco's own functions on it. It cannot use `Tera::one_off`, because that renders with only Tera's built-ins — and `get_env(name=.., default=..)`, used throughout the shipped config files, is **not** one of them: Tera 1 shipped it, Tera 2 dropped it, and Loco now registers its own (`src/tera.rs:16-32`). Tera's native `{{ }}`/`{% %}` delimiters still render for backward compatibility, but they are deprecated (they are YAML flow-mapping syntax, not a plain scalar, so a YAML formatter can rewrite and break them — see [The configuration model](/docs/explanation/configuration-model#the-yaml-is-templated-first-a-config-file-second)) and log a warning when used.
- Parse failures raise `Error::YAMLFile(err, path)` (`src/config/mod.rs:172-173`).
- `Config` implements `Display` by dumping itself back to YAML (`src/config/mod.rs:191-196`).
- `Config::get_jwt_config(&self) -> Result<&JWT>` (`src/config/mod.rs:180-188`) returns an error if `auth` or `auth.jwt` is absent.

### Environment resolution

`environment::resolve_from_env()` (`src/environment.rs:32-38`) picks the active environment name with this precedence:

1. `LOCO_ENV`
2. `RAILS_ENV`
3. `NODE_ENV`
4. fallback: `"development"` (`DEFAULT_ENVIRONMENT`, `src/environment.rs:21`)

## Top-level `Config`

`struct Config` — `src/config/mod.rs:62-92`. Every field is a top-level YAML key.

| Key | Type | Required? | Notes |
|---|---|---|---|
| `logger` | [`Logger`](#logger) | required | `mod.rs:64` |
| `server` | [`Server`](#server) | required | `mod.rs:65` |
| `database` | [`Database`](#database) | required, only when the `with-db` feature is enabled | `#[cfg(feature = "with-db")]`, `mod.rs:66-67` |
| `cache` | [`CacheConfig`](#cache) | optional — `#[serde(default)]`, defaults to `Null` | `mod.rs:68-69` |
| `queue` | `Option<`[`QueueConfig`](#queue)`>` | optional | `mod.rs:70` |
| `auth` | `Option<`[`Auth`](#auth)`>` | optional | `mod.rs:71` |
| `workers` | [`Workers`](#workers) | optional — `#[serde(default)]` | `mod.rs:72-73` |
| `mailer` | `Option<`[`Mailer`](#mailer)`>` | optional | `mod.rs:74` |
| `initializers` | `Option<Initializers>` (= `Option<BTreeMap<String, serde_json::Value>>`) | optional | `mod.rs:75`, type alias at `mod.rs:106` |
| `settings` | `Option<serde_json::Value>` | optional — `#[serde(default)]` | `mod.rs:88-89`; free-form app settings, surfaced at `ctx.config.settings` |
| `scheduler` | `Option<scheduler::Config>` | optional | `mod.rs:91`; struct owned by the scheduler area, not detailed here |

## `auth`

`struct Auth` — `src/config/auth.rs:13-17`.

```yaml
auth:
  jwt:
    location:              # optional, default: Bearer
      from: Bearer          # or: {from: Query, name: <string>} / {from: Cookie, name: <string>}
    secret: <base64 secret> # required — must be valid base64
    expiration: 604800      # required, u64 seconds (e.g. 7 days)
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `auth.jwt` | `Option<JWT>` | optional | `auth.rs:16` |
| `auth.jwt.location` | `Option<JWTLocationConfig>` | optional, default: `Bearer` (resolved by `get_jwt_locations`, `src/controller/extractor/auth.rs:181-189`) | `auth.rs:24` |
| `auth.jwt.secret` | `String` | required | `auth.rs:26`. **Must be valid base64** — the JWT signer/verifier call `EncodingKey`/`DecodingKey::from_base64_secret` (`src/auth/jwt.rs:83,108`); a non-base64 string fails at token generation/validation time, not at config load time |
| `auth.jwt.expiration` | `u64` (seconds) | required | `auth.rs:28` |

`JWTLocationConfig` (`auth.rs:61-106`) accepts either form:
- `Single(JWTLocation)` — a single location map
- `Multiple(Vec<JWTLocation>)` — a YAML list of location maps, tried in order until one yields a token

`#[serde(untagged)]` applies to `Serialize` only. `Deserialize` is hand-written and dispatches on the input's shape — a map is a single location, a sequence is a fallback list — so a malformed entry reports the actual reason (a wrong-case `from: cookie`, a `Cookie` missing its `name`) instead of an untagged enum's opaque "did not match any variant".

`JWTLocation` (`#[serde(tag = "from")]`, `auth.rs:35-44`):

| Variant | YAML shape | Notes |
|---|---|---|
| `Bearer` | `from: Bearer` | reads the `Authorization: Bearer <token>` header |
| `Query { name }` | `from: Query`<br>`name: <string>` | reads a query-string parameter |
| `Cookie { name }` | `from: Cookie`<br>`name: <string>` | reads a cookie |

Related, not YAML-configurable: default signing algorithm is **HS512** (`JWT_ALGORITHM`, `src/auth/jwt.rs:13`), overridable in code via `JWT::algorithm(..)` (`jwt.rs:51`), not via config.

## `server`

`struct Server` — `src/config/server.rs:29-45`.

```yaml
server:
  binding: localhost     # optional, default "localhost"
  port: 5150             # required
  host: http://localhost # required
  ident: <string>        # optional — overrides the `Server` response header
  middlewares: {}        # optional, default {} — see the middleware catalog reference
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `server.binding` | `String` | optional — `#[serde(default = "default_binding")]` → `"localhost"` | `server.rs:33-34,47-49` |
| `server.port` | `i32` | required | `server.rs:36` |
| `server.host` | `String` | required | `server.rs:38` |
| `server.ident` | `Option<String>` | optional | `server.rs:40`. When set, replaces the `Server` header value |
| `server.middlewares` | `middleware::Config` | optional — `#[serde(default)]` | `server.rs:44`. Struct is owned by the middleware area; see the middleware catalog reference for every middleware's keys |

`Server::full_url() -> String` returns `"{host}:{port}"` (`server.rs:52-55`).

## `workers`

`struct Workers` — `src/config/server.rs:64-68`.

```yaml
workers:
  mode: BackgroundQueue  # optional, default BackgroundQueue
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `workers.mode` | `WorkerMode` | optional — `Workers` derives `Default` | `server.rs:67` |

`WorkerMode` (`server.rs:71-83`):

| Variant | Default? | Notes |
|---|---|---|
| `BackgroundQueue` | yes | Workers run asynchronously via a queue backend. **Requires a configured `queue`** |
| `ForegroundBlocking` | no | Workers run in-process and block the caller until the task completes |
| `BackgroundAsync` | no | Workers run asynchronously in-process (async task, no external queue) |

## `database`

`struct Database` — `src/config/database.rs:22-84`. Only present/required when the `with-db` feature is enabled.

```yaml
database:
  uri: postgres://root:12341234@localhost:5432/myapp_development  # required
  enable_logging: true       # required — SQLx statement logging
  min_connections: 1         # required
  max_connections: 1         # required
  connect_timeout: 500       # required, milliseconds
  idle_timeout: 500          # required, milliseconds
  acquire_timeout: 500       # optional, milliseconds
  auto_migrate: true         # optional, default false
  dangerously_truncate: false# optional, default false
  dangerously_recreate: false# optional, default false
  run_on_start: <sql/pragma> # optional
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `database.uri` | `String` | required | `database.rs:28`. E.g. `postgres://...` or `sqlite://db.sqlite?mode=rwc` |
| `database.enable_logging` | `bool` | required | `database.rs:31` — enables SQLx statement logging |
| `database.min_connections` | `u32` | required | `database.rs:34` |
| `database.max_connections` | `u32` | required | `database.rs:37` |
| `database.connect_timeout` | `u64` (ms) | required | `database.rs:40` |
| `database.idle_timeout` | `u64` (ms) | required | `database.rs:43` |
| `database.acquire_timeout` | `Option<u64>` (ms) | optional | `database.rs:46` |
| `database.auto_migrate` | `bool` | optional — `#[serde(default)]` | `database.rs:51-52`. Runs pending migrations on boot; recommended for development, discouraged in production |
| `database.dangerously_truncate` | `bool` | optional — `#[serde(default)]` | `database.rs:56-57`. Deletes row data on boot; typically used in `test` |
| `database.dangerously_recreate` | `bool` | optional — `#[serde(default)]` | `database.rs:63-64`. Drops and recreates schema on boot |
| `database.run_on_start` | `Option<String>` | optional | `database.rs:83`. Arbitrary SQL/PRAGMA statements executed once the connection is established. For SQLite, if unset, Loco applies its own PRAGMA defaults (`foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size=134217728`, `journal_size_limit=67108864`, `cache_size=2000`, `busy_timeout=5000`) |

Note: the `db_min_conn()=1` / `db_max_conn()=20` / `db_connect_timeout()=500` / `db_idle_timeout()=500` helper functions in `database.rs:86-100` are **not** defaults for `Database` itself (whose numeric fields have no `#[serde(default)]` and are required) — they are reused as the `#[serde(default = ...)]` values for the Postgres/Sqlite [`queue`](#queue) configs below.

## `logger`

`struct Logger` — `src/config/logger.rs:21-49`.

```yaml
logger:
  enable: true            # required
  pretty_backtrace: false # optional, default false
  level: debug            # required — off|trace|debug|info|warn|error
  format: compact         # required — compact|pretty|json
  override_filter: <str>  # optional — EnvFilter directive string
  file_appender:          # optional
    enable: true           # required within block
    non_blocking: false    # optional, default false, within block
    level: info            # required within block
    format: json           # required within block
    rotation: daily        # required within block — minutely|hourly|daily|never
    dir: ./logs            # optional, default "./logs"
    filename_prefix: <s>   # optional
    filename_suffix: <s>   # optional
    max_log_files: 7       # required within block
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `logger.enable` | `bool` | required | `logger.rs:24` |
| `logger.pretty_backtrace` | `bool` | optional — `#[serde(default)]` | `logger.rs:28-29`. When `true`, forces nicely-formatted backtraces (development-friendly); turn off in performance-sensitive production deployments |
| `logger.level` | `logger::LogLevel` | required | `logger.rs:34`. Variants: `off`, `trace`, `debug`, `info` (enum's own `#[default]`, but the field itself has no `#[serde(default)]` so it must be present in YAML), `warn`, `error` (`src/logger.rs:15-36`) |
| `logger.format` | `logger::Format` | required | `logger.rs:39`. Variants: `compact` (`#[default]`), `pretty`, `json` (`src/logger.rs:39-48`) |
| `logger.override_filter` | `Option<String>` | optional | `logger.rs:45`. A `tracing-subscriber` `EnvFilter` directive string |
| `logger.file_appender` | `Option<LoggerFileAppender>` | optional | `logger.rs:48` |
| `logger.file_appender.enable` | `bool` | required within block | `logger.rs:54` |
| `logger.file_appender.non_blocking` | `bool` | optional — `#[serde(default)]` | `logger.rs:57-58` |
| `logger.file_appender.level` | `logger::LogLevel` | required within block | `logger.rs:63` |
| `logger.file_appender.format` | `logger::Format` | required within block | `logger.rs:68` |
| `logger.file_appender.rotation` | `logger::Rotation` | required within block | `logger.rs:71`. Variants: `minutely`, `hourly` (`#[default]`), `daily`, `never` (`src/logger.rs:51-62`) |
| `logger.file_appender.dir` | `Option<String>` | optional, defaults to `"./logs"` when unset (applied at file-appender init, `src/logger.rs:113`) | `logger.rs:76` |
| `logger.file_appender.filename_prefix` | `Option<String>` | optional | `logger.rs:79` |
| `logger.file_appender.filename_suffix` | `Option<String>` | optional | `logger.rs:82` |
| `logger.file_appender.max_log_files` | `usize` | required within block | `logger.rs:85` |

## `mailer`

`struct Mailer` — `src/config/mailer.rs:29-35`.

```yaml
# development: capture instead of sending
mailer:
  stub: false          # optional, default false
  smtp:
    enable: true       # required
    host: localhost    # required
    port: 1025         # required
    secure: false      # required — legacy shorthand, see below

# production: implicit TLS on port 465 (SMTPS)
mailer:
  smtp:
    enable: true
    host: smtp.example.com
    port: 465
    tls: implicit       # overrides `secure`; see below
    auth:
      user: postmaster@mg.example.com
      password: "<%= get_env(name='SMTP_PASSWORD') %>"
    hello_name: <string> # optional — EHLO client id
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `mailer.stub` | `bool` | optional — `#[serde(default)]` | `mailer.rs:33-34`. When `true`, mail is captured rather than sent |
| `mailer.smtp` | `Option<SmtpMailer>` | optional | `mailer.rs:31` |
| `mailer.smtp.enable` | `bool` | required | `mailer.rs:55` |
| `mailer.smtp.host` | `String` | required | `mailer.rs:57` |
| `mailer.smtp.port` | `u16` | required | `mailer.rs:59` |
| `mailer.smtp.secure` | `bool` | required | `mailer.rs:65`. Legacy shorthand: `true` selects `STARTTLS` (port 587), `false` selects cleartext |
| `mailer.smtp.tls` | `Option<MailerTls>` | optional — `#[serde(default)]` | `mailer.rs:69`. **When set, overrides `secure`.** Variants (`#[serde(rename_all = "lowercase")]`, `mailer.rs:38-50`): `starttls` (opportunistic TLS, port 587 — what `secure: true` selects), `implicit` (TLS from the first byte, SMTPS, port 465 — required by providers that only accept implicit TLS), `none` (cleartext) |
| `mailer.smtp.auth` | `Option<MailerAuth>` | optional | `mailer.rs:71` |
| `mailer.smtp.auth.user` | `String` | required within block | `mailer.rs:93` |
| `mailer.smtp.auth.password` | `String` | required within block | `mailer.rs:95` |
| `mailer.smtp.hello_name` | `Option<String>` | optional | `mailer.rs:73`. `EHLO` client identifier, sent instead of the hostname |

Effective TLS mode is resolved by `SmtpMailer::tls_mode()` (`mailer.rs:76-87`): if `tls` is set, it wins outright; otherwise `secure: true` → `Starttls`, `secure: false` → `None`.

## `queue`

`enum QueueConfig` — `src/config/queue.rs:5-14`, `#[serde(tag = "kind")]` with variants `Redis`, `Postgres`, `Sqlite`.

```yaml
# kind: Redis
queue:
  kind: Redis
  uri: redis://127.0.0.1                # required
  dangerously_flush: false              # optional, default false
  queues: [high, low]                   # optional — priority order, first = most important
  num_workers: 2                        # optional, default 2
  # reaper:                             # optional, disabled by default (opt-in)
  #   age_minutes: 10                   # requeue jobs stuck in `processing` for longer than this
  #   interval_seconds: 60              # optional, default 60 — how often to sweep

# kind: Postgres
queue:
  kind: Postgres
  uri: postgres://...                   # required
  dangerously_flush: false              # optional, default false
  enable_logging: false                 # optional, default false
  max_connections: 20                   # optional, default 20
  min_connections: 1                    # optional, default 1
  connect_timeout: 500                  # optional, default 500 (ms)
  idle_timeout: 500                     # optional, default 500 (ms)
  poll_interval_sec: 1                  # optional, default 1
  num_workers: 2                        # optional, default 2
  # reaper:                             # optional, disabled by default (opt-in)
  #   age_minutes: 10                   # requeue jobs stuck in `processing` for longer than this
  #   interval_seconds: 60              # optional, default 60 — how often to sweep

# kind: Sqlite (same shape as Postgres)
queue:
  kind: Sqlite
  uri: sqlite://...
  poll_interval_sec: 1                  # optional, default 1 (own default fn)
  # ...remaining keys identical to Postgres, including the optional `reaper`
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `queue.kind` | tag: `Redis` \| `Postgres` \| `Sqlite` | required | `queue.rs:6-14` |
| **Redis** (`RedisQueueConfig`, `queue.rs:16-28`) | | | |
| `queue.uri` | `String` | required | `queue.rs:18` |
| `queue.dangerously_flush` | `bool` | optional — `#[serde(default)]` | `queue.rs:20` |
| `queue.queues` | `Option<Vec<String>>` | optional | `queue.rs:24`. Declares named priority queues; first entry is most important |
| `queue.num_workers` | `u32` | optional, default `2` (`num_workers()`) | `queue.rs:26-27` |
| `queue.reaper` | `Option<ReaperConfig>` | optional, default `None` (disabled) | `queue.rs:29-31`. See below |
| **Postgres** (`PostgresQueueConfig`, `queue.rs:30-57`) | | | |
| `queue.uri` | `String` | required | `queue.rs:32` |
| `queue.dangerously_flush` | `bool` | optional, default `false` | `queue.rs:34-35` |
| `queue.enable_logging` | `bool` | optional, default `false` | `queue.rs:37-38` |
| `queue.max_connections` | `u32` | optional, default `20` (`db_max_conn()`) | `queue.rs:40-41` |
| `queue.min_connections` | `u32` | optional, default `1` (`db_min_conn()`) | `queue.rs:43-44` |
| `queue.connect_timeout` | `u64` (ms) | optional, default `500` (`db_connect_timeout()`) | `queue.rs:46-47` |
| `queue.idle_timeout` | `u64` (ms) | optional, default `500` (`db_idle_timeout()`) | `queue.rs:49-50` |
| `queue.poll_interval_sec` | `u32` | optional, default `1` (`pgq_poll_interval()`) | `queue.rs:52-53` |
| `queue.num_workers` | `u32` | optional, default `2` | `queue.rs:55-56` |
| `queue.reaper` | `Option<ReaperConfig>` | optional, default `None` (disabled) | `queue.rs:57-59`. See below |
| **Sqlite** (`SqliteQueueConfig`, `queue.rs:59-86`) | | | |
| — | identical fields to Postgres, including `queue.reaper` | | `poll_interval_sec` defaults via its own `sqlt_poll_interval()=1` (`queue.rs:81-82,92-94`); all other defaults are shared with Postgres via the same `db_*` helper functions |
| **`ReaperConfig`** (`queue.rs`, all three backends) | | | Opt-in visibility-timeout reaper: when set, the queue provider spawns a background task that periodically requeues jobs stuck in `processing` (e.g. after a worker crash), reusing the same logic as `cargo loco jobs requeue`. Leaving it unset keeps the previous behavior — no automatic requeue. |
| `queue.reaper.age_minutes` | `i64` | required (only if `reaper` is set) | Requeue jobs that have been `processing` for longer than this many minutes |
| `queue.reaper.interval_seconds` | `u64` | optional, default `60` (`default_reaper_interval_seconds()`) | How often the reaper sweeps for stale jobs |

## `cache`

`enum CacheConfig` — `src/config/cache.rs:4-16`, `#[serde(tag = "kind")]`. **Default variant: `Null`** (`#[default]`, `cache.rs:14-15`) — this is what `Config.cache`'s `#[serde(default)]` produces when the `cache` key is omitted entirely.

```yaml
cache:
  kind: InMem              # requires the `cache_inmem` feature
  max_capacity: 33554432   # optional, default 33554432 bytes (32 MiB)

# --- or ---
cache:
  kind: Redis              # requires the `cache_redis` feature
  uri: redis://...         # required
  max_size: 100            # required — max pool connections

# --- or (default) ---
cache:
  kind: "Null"             # no-op cache; used when `cache` key is omitted
                           # must be quoted — bare Null is YAML's null, not the string
```

| Key | Type | Required? | Notes |
|---|---|---|---|
| `cache.kind` | tag: `InMem` \| `Redis` \| `"Null"` | required if `cache` present | `cache.rs:6-16`. `Null` must be written quoted: unquoted, YAML resolves it to null and the tagged enum fails to deserialize |
| **InMem** (`InMemCacheConfig`, `cache.rs:18-22`) — feature-gated on `cache_inmem` | | | |
| `cache.max_capacity` | `u64` | optional, default `33554432` (`32 * 1024 * 1024`, `cache_in_mem_max_capacity()`) | `cache.rs:20-21,24-26` |
| **Redis** (`RedisCacheConfig`, `cache.rs:28-33`) — feature-gated on `cache_redis` | | | |
| `cache.uri` | `String` | required | `cache.rs:30` |
| `cache.max_size` | `u32` | required — max pool connections | `cache.rs:32` |
| **Null** | (no fields) | — | default no-op cache |

If the corresponding feature (`cache_inmem` / `cache_redis`) is not compiled in, that `kind` value will fail to deserialize.

## `initializers`, `settings`, `scheduler`

- `initializers: Option<BTreeMap<String, serde_json::Value>>` (`mod.rs:75,106`) — a free-form map consumed by app initializers (e.g. an `oauth2` initializer reading `initializers.oauth2`). Keys and shapes are defined by whichever initializer reads them, not by `Config` itself.
- `settings: Option<serde_json::Value>` (`mod.rs:88-89`) — arbitrary app-defined settings, deserialize your own type from `ctx.config.settings`.
- `scheduler: Option<scheduler::Config>` (`mod.rs:91`) — struct and keys owned by the scheduler area; not detailed on this page.

## Environment variables

| Variable | Purpose | Source |
|---|---|---|
| `LOCO_ENV` | Selects the active environment; highest precedence | `src/environment.rs:22,34` |
| `RAILS_ENV` | Falls back to this if `LOCO_ENV` is unset | `src/environment.rs:23,35` |
| `NODE_ENV` | Falls back to this if both above are unset | `src/environment.rs:24,36` |
| `LOCO_CONFIG_FOLDER` | Overrides the `config/` folder Loco loads from | `src/env_vars.rs:16`, read in `Environment::load` (`src/environment.rs:59-64`) |
| `LOCO_DATA` | Data folder path | `src/env_vars.rs:20` |
| `LOCO_POSTGRES_DB_OPTIONS` | Extra Postgres connection options (only meaningful with `with-db`) | `src/env_vars.rs:8` |
| `SCHEDULER_CONFIG` | Path to the scheduler config file | `src/env_vars.rs:18` |
| `RUST_BACKTRACE` | Effectively forced to `1` when `logger.pretty_backtrace: true` | logger init |
| any name passed to `get_env(name=.., default=..)` in a config YAML file | Injected into the rendered YAML at load time by Loco's own `get_env` Tera function | `src/tera.rs:16-32` |

Secrets (JWT `secret`, SMTP `password`, database `uri` credentials) are plain `String` fields with no dedicated vault type; the convention is to inject them via `<%= get_env(name="...") %>` at config-load time rather than hardcoding them in the YAML file.
