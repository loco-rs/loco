---
title: Feature flags
description: "The complete loco-rs Cargo feature matrix: defaults, what each flag enables, and how flags interact."
sidebar:
  order: 4
---

`loco-rs` gates most of its optional functionality behind Cargo features, declared in root `Cargo.toml:27-64`. This page is the exhaustive matrix — every flag, its default state, what it turns on, and how flags interact with each other and with `cargo loco`.

## Defaults

```toml
default = ["auth", "cli", "with-db", "cache_inmem", "worker"]
```

A plain `loco-rs = "..."` dependency (no `default-features = false`) pulls in JWT auth, the `cargo loco` CLI, Sea-ORM database support, the in-memory cache, and the Postgres/SQLite-backed queue workers. The Redis-backed queue worker (`worker_redis`) is *not* in the default set — opt in explicitly if your app uses a Redis queue.

## The matrix

| Flag | Default | Enables (deps / sub-features) | Purpose |
|---|---|---|---|
| `auth` | **ON** | `dep:jsonwebtoken`, `jsonwebtoken/rust_crypto` | JWT authentication. Selects `jsonwebtoken`'s pure-Rust `rust_crypto` backend (jsonwebtoken 10 no longer bundles a crypto backend by default), so the flag stays self-contained and needs no C toolchain, even when enabled alone with `default-features = false`. |
| `cli` | **ON** | `dep:clap` | Enables the `cargo loco` runtime CLI (`src/cli.rs`). |
| `with-db` | **ON** | `dep:sea-orm`, `dep:sea-orm-migration`, `dep:sqlx`, `loco-gen/with-db` | Sea-ORM 2.0.0-rc database support. Gates the `db` CLI subcommand and the DB-dependent generators (`model`, `migration`, `scaffold`). |
| `testing` | off | `dep:axum-test`, `dep:scraper`, `dep:tree-fs` | Test harness utilities. Also the feature set built for docs.rs (`[package.metadata.docs.rs] features = ["testing"]`, `Cargo.toml:211-212`) and used by the crate's own `dev-dependencies`. |
| `cache_inmem` | **ON** | `dep:moka` | In-memory cache backend. |
| `cache_redis` | off | `dep:bb8-redis`, `dep:bb8` | Redis-backed cache pool. |
| `worker` | **ON** | `dep:sqlx`, `dep:ulid` | Background job queue/workers, Postgres and SQLite backends. Which one runs is chosen at runtime by `queue.kind` in config (`Postgres` or `Sqlite`), not by a separate feature per database. |
| `worker_redis` | off | `worker`, `dep:redis` | Adds the Redis-backed queue backend on top of `worker` (implies it). Enable this if your app's `queue.kind` is `Redis`. |
| `all_storage` | off | `storage_aws_s3` + `storage_azure` + `storage_gcp` | Umbrella flag — turns on every cloud storage backend at once. |
| `storage_aws_s3` | off | `opendal/services-s3` | AWS S3 storage backend. |
| `storage_azure` | off | `opendal/services-azblob` | Azure Blob storage backend. |
| `storage_gcp` | off | `opendal/services-gcs` | Google Cloud Storage backend. |
| `embedded_assets` | off | (empty — build-time flag) | Embeds the app's `assets/` directory into the compiled binary and swaps the view-engine's asset-loading path accordingly, instead of reading assets from disk at runtime. |

Source: root `Cargo.toml:27-64`.

## Interactions

- **`worker` unlocks the `jobs` subcommand.** `cargo loco jobs` (and its `cancel`/`tidy`/`purge`/`dump`/`import`/`requeue` subcommands) is compiled whenever the `worker` feature is enabled (`#[cfg(feature = "worker")]`, `src/cli.rs:27`). The same cfg gates the `JobStatus` import used by the jobs machinery. Since `worker_redis` implies `worker`, the `jobs` CLI is available for any queue backend — Redis, Postgres, or SQLite.
- **`debug_assertions` (not a Cargo feature) gates `generate` and `db entities`.** The `cargo loco generate` subcommand and the `db entities` subcommand are compiled only in debug builds (`#[cfg(debug_assertions)]`, `src/cli.rs:29, 140, 173`). They are unavailable in `--release` builds regardless of which Cargo features are on.
- **`all_storage` is a pure umbrella.** It has no dependency of its own; it just turns on `storage_aws_s3`, `storage_azure`, and `storage_gcp` together.
- **`auth` selects `jsonwebtoken/rust_crypto`.** Because jsonwebtoken 10 unbundled its crypto backend, `auth` explicitly enables the `rust_crypto` sub-feature so JWT support keeps working without requiring a system C toolchain (e.g. OpenSSL).
- **`with-db` is a prerequisite, not an implication.** Enabling `worker` does not itself pull in `with-db`; the two are independent flags that happen to share the `sqlx` dependency.
- **The queue backend is chosen at runtime, not by feature flag.** `worker` builds in the Postgres and SQLite queue providers; which one actually runs is decided by `queue.kind` (`Postgres` or `Sqlite`) in your app config. `worker_redis` adds the Redis provider, selected the same way with `queue.kind: Redis`. See [Choose a queue backend](@/docs/how-to/choose-queue-backend.md).

## Disabling defaults

To opt out of the default set (e.g. a DB-less app), depend with `default-features = false` and re-list only the flags you want:

```toml
loco-rs = { version = "...", default-features = false, features = ["cli"] }
```

This is the pattern the `loco new` generator itself uses when the app is created without a database (see the CLI reference's app-creation flow): it emits `default-features = false` with `features = ["cli"]`, plus `worker_redis` if a Redis-backed queue was selected, or `worker` if a Postgres- or SQLite-backed queue was selected.
