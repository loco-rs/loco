# Loco Feature-Matrix Cleanup (WS3) — Design

**Status:** approved (design), pending spec review
**Date:** 2026-07-06
**Program:** "dark areas" WS3 (Area 3 — Cargo feature matrix). Part of the 0.17.0
BREAKING release (already breaking via Sea-ORM 2.0). Breaking feature-flag
changes are acceptable and expected.

## Goal

Simplify and de-confuse Loco's Cargo feature matrix for 1.0: collapse the three
illusory background-queue backend flags into a coherent `worker` / `worker_redis`
pair, drop a dead feature, make the queue backends discoverable in the wizard,
honestly name the auth feature, and fix the silently-broken `embedded_assets`
feature.

## Background (verified facts, from the dark-areas audit + this session)

- `integration_test` is **dead** — zero `.rs` references anywhere.
- `bg_pg` / `bg_sqlt` gate ~150 lines of thin wrapper, **not** driver weight:
  `sqlx` (with both `postgres` + `sqlite`) and `sea-orm` (both drivers) compile
  unconditionally whenever `with-db` is on. Only `bg_redis` pulls a real extra
  crate (`dep:redis`).
- The queue backend is already **runtime-dispatched**: `create_queue_provider`
  (`src/bgworker/mod.rs:756`) matches on `config.queue` (`QueueConfig::Redis` /
  `Postgres` / `Sqlite`), each arm compile-gated by the matching `bg_*` feature.
  So collapsing the flags is a mechanical cfg rename, not an architecture change.
- `bg_*` cfg sites span ~8 files: `src/bgworker/mod.rs`, `src/cli.rs`,
  `src/controller/mod.rs`, `src/controller/monitoring.rs`, `src/errors.rs`,
  `src/tests_cfg/queue.rs` (plus `bgworker/{pg,sql,redis}.rs` module gating).
- The wizard only ever emits a **Redis** queue; pg/sqlite queues are reachable
  only by hand-editing YAML. Feature emission lives in
  `loco-new/src/settings.rs:62-70` (db app → `Features::default()`; non-db app →
  `disable_features()` + push `bg_redis` when `Queue`).
- `embedded_assets` is real in the lib (embeds `assets/`) but never wired into
  `loco new`, and silently 404s when combined with a clientside/React app (no
  `assets/` dir exists to embed).
- `auth_jwt` also gates the non-JWT `ApiToken` extractor (misnamed coupling).
  `jsonwebtoken` is the only crate weight it carries.

## Decisions

1. **Scope = everything** (all five task groups below).
2. **`worker` = pg + sqlite** (no extra crate; self-contained via `dep:sqlx` +
   `dep:ulid`). **`worker_redis` = `worker` + `dep:redis`**. Redis is an opt-in
   add-on, not in `default`.
3. **`auth_jwt` → `auth`: pure rename** (one feature gates JWT + `ApiToken`).
   No split — the ApiToken-only-without-jsonwebtoken win is niche (YAGNI).
4. **`embedded_assets`: fix + guard** (keep for serverside; wizard-exposed for
   serverside only; hard error on `embedded_assets + clientside`).

## Target feature set

```toml
default = ["auth", "cli", "with-db", "cache_inmem", "worker"]

auth          = ["dep:jsonwebtoken", "jsonwebtoken/rust_crypto"]  # renamed from auth_jwt
worker        = ["dep:sqlx", "dep:ulid"]        # Postgres + SQLite queue backends
worker_redis  = ["worker", "dep:redis"]         # + Redis backend
# removed: bg_redis, bg_pg, bg_sqlt, integration_test
# unchanged: cli, testing, with-db, cache_inmem, cache_redis, storage_*, embedded_assets
```

## Task groups (independently testable, in build order)

### ① Drop `integration_test`
Remove the feature line from `Cargo.toml`. Grep-verify zero `.rs`/template
references remain. No other change.

### ② `auth_jwt` → `auth` (pure rename)
- `Cargo.toml`: rename the feature key; update `default`.
- Rewrite every `#[cfg(feature = "auth_jwt")]` → `#[cfg(feature = "auth")]`
  (grep the tree; includes lib + `loco-gen` + docs references).
- `loco-new` base_template / settings: any emitted `auth_jwt` → `auth`.
- Update CHANGELOG breaking-changes + migration guide + AGENTS/docs mentions.

### ③ Collapse `bg_*` → `worker` / `worker_redis`
- `Cargo.toml`: replace the three `bg_*` features with `worker` / `worker_redis`
  (defs above); update `default`; make `dep:redis` / `dep:ulid` optional as
  needed. `sqlx` is already an optional dep pulled by `with-db`/`worker`.
- cfg rewrites (mechanical):
  - `feature = "bg_pg"` → `feature = "worker"`
  - `feature = "bg_sqlt"` → `feature = "worker"`  (merge now-duplicate arms; the
    runtime `QueueConfig::Postgres` vs `::Sqlite` match still distinguishes them)
  - `feature = "bg_redis"` → `feature = "worker_redis"`
  - `any(feature="bg_pg", feature="bg_sqlt", feature="bg_redis")` → `feature = "worker"`
    (`worker_redis` implies `worker`)
- `src/bgworker/mod.rs` module gating: `pg`/`sql` mods under `worker`, `redis`
  mod under `worker_redis`.
- Verify: `cargo check -p loco-rs` for feature combos `worker`, `worker_redis`,
  and neither (queue disabled → the no-op provider path still compiles).

### ④ Wire pg/sqlite queues into the wizard
- Reshape `BackgroundOption` (`loco-new/src/wizard.rs`) from
  `Async | Queue | Blocking` to expose the durable-queue backend:
  `Async | QueueRedis | QueuePostgres | QueueSqlite | Blocking`
  (display strings updated; `serde`/`strum` names chosen to stay descriptive).
- Feature + config emission (`loco-new/src/settings.rs` + `config/*.yaml.t`),
  per selection:

  | BackgroundOption | loco-rs feature to ensure | `workers.mode` | `queue.kind` |
  |---|---|---|---|
  | Async            | (none — in-process)        | `BackgroundAsync`    | (no queue block) |
  | QueueRedis       | `worker_redis`             | `BackgroundQueue`    | `Redis`   |
  | QueuePostgres    | `worker`                   | `BackgroundQueue`    | `Postgres`|
  | QueueSqlite      | `worker`                   | `BackgroundQueue`    | `Sqlite`  |
  | Blocking         | (none)                     | `ForegroundBlocking` | (no queue block) |

  - db apps use `Features::default()` (which now includes `worker`); a Redis
    selection additionally pushes `worker_redis`.
  - non-db apps (`disable_features()`) push `worker` or `worker_redis` only for
    the Queue* selections.
  - `config/development.yaml.t` + `test.yaml.t`: render the `queue:` block from
    the selected backend (currently hard-codes `kind: Redis`); omit the block for
    Async/Blocking.

### ⑤ `embedded_assets` fix + guard
- Add a wizard toggle ("Embed static assets into the binary?") offered **only**
  when the asset choice is Serverside.
- When enabled: push `embedded_assets` into the emitted feature list and keep the
  `assets/` dir (already copied for serverside).
- Guard: `embedded_assets` selected together with a Clientside app is a **hard
  error** at wizard/generation time (no `assets/` dir → the silent-404 bug).
- Leave the lib feature itself unchanged (it works for serverside).

## Testing strategy

The now-green `loco-new` wizard matrix (`tests/wizard/new.rs`
`test_starter_combinations`) is the harness.

- Extend `BackgroundOption` coverage: add cases exercising `QueuePostgres` /
  `QueueSqlite` (→ `worker`) and `QueueRedis` (→ `worker_redis`), at least on the
  Sqlite-db combo. (Redis runtime needs a server; the *compile+clippy* of a
  `worker_redis` app is the valuable check — a full Redis run stays behind the
  Docker-gated path.)
- Add a Serverside + `embedded_assets` combo (compiles, clippy clean).
- loco-rs feature-combo checks: `cargo check`/`clippy` with `worker`,
  `worker_redis`, and queue-disabled, so no cfg arm rots.
- Update loco-gen + loco-rs snapshots touched by the cfg renames.

## Non-goals

- No change to the runtime queue dispatch or the worker execution model.
- No `auth` split (ApiToken vs JWT) — pure rename only.
- No new queue backends.
- Full Redis-runtime wizard combos remain Docker-gated (not added to the default
  fast matrix).

## Migration notes (for CHANGELOG + upgrade guide)

- `auth_jwt` feature renamed to `auth`.
- `bg_redis` / `bg_pg` / `bg_sqlt` replaced by `worker` (pg+sqlite) and
  `worker_redis` (adds Redis). Apps using a Redis queue must switch
  `bg_redis` → `worker_redis`; pg/sqlite queue users switch to `worker`.
- `integration_test` feature removed (was dead).
- `default` feature set is now `["auth", "cli", "with-db", "cache_inmem", "worker"]`
  (Redis no longer default; add `worker_redis` for a Redis queue).
