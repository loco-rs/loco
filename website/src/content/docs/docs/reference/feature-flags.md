---
title: Feature flags
description: "The complete loco-rs Cargo feature matrix: defaults, what each flag enables, and how flags interact."
sidebar:
  order: 4
---

`loco-rs` gates most of its optional functionality behind Cargo features, declared in root `Cargo.toml:41-109`. This page is the exhaustive matrix — every flag, its default state, what it turns on, and how flags interact with each other and with `cargo loco`.

## Defaults

```toml
default = [
    "auth", "cli", "with-db", "cache_inmem", "worker",
    "db-sqlite", "db-postgres", "compression", "generators",
]
```

A plain `loco-rs = "..."` dependency (no `default-features = false`) pulls in JWT auth, the `cargo loco` CLI, Sea-ORM database support, the in-memory cache, the Postgres/SQLite drivers and queue workers, gzip for the `compression` middleware, and `cargo loco generate`. The Redis-backed queue worker (`worker_redis`) is *not* in the default set — opt in explicitly if your app uses a Redis queue.

## The matrix

| Flag | Default | Enables (deps / sub-features) | Purpose |
|---|---|---|---|
| `auth` | **ON** | `dep:hmac`, `dep:sha2`, `dep:base64` | JWT authentication (HS256/HS384/HS512). Pure Rust, so the flag stays self-contained and needs no C toolchain, even when enabled alone with `default-features = false`. |
| `cli` | **ON** | `dep:clap` | Enables the `cargo loco` runtime CLI (`src/cli.rs`). |
| `with-db` | **ON** | `dep:sea-orm`, `dep:sea-orm-migration`, `dep:sqlx`, `loco-gen?/with-db` | Sea-ORM 2.0 database support. Gates the `db` CLI subcommand and the DB-dependent generators (`model`, `migration`, `scaffold`). Needs a driver: `db-postgres`, `db-sqlite`, or both. |
| `db-sqlite` | **ON** | `sea-orm?/sqlx-sqlite`, `sea-orm-migration?/sqlx-sqlite`, `sqlx?/sqlite` | SQLite SeaORM/sqlx drivers and the SQLite queue worker backend. Disable with `default-features = false` on Postgres-only apps to skip `libsqlite3-sys` compile cost. |
| `db-postgres` | **ON** | `sea-orm?/sqlx-postgres`, `sea-orm-migration?/sqlx-postgres`, `sqlx?/postgres` | Postgres SeaORM/sqlx drivers, the Postgres queue worker backend, and the Postgres test helpers. Leave it out with `default-features = false` on SQLite-only apps to skip `sqlx-postgres` and its dependencies. |
| `compression` | **ON** | `tower-http/compression-gzip` | The `compression` middleware, gzip only. Enough for the clients that send `Accept-Encoding: gzip`, which is practically all of them. |
| `compression-full` | off | `compression`, `tower-http/compression-full` | The `compression` middleware with brotli, deflate, gzip and zstd. zstd is a C build of libzstd. |
| `generators` | **ON** | `dep:loco-gen` | `cargo loco generate` (debug builds only). `loco-gen` brings a second, Tera 1, template engine through `rrgen`; leaving the feature out keeps it out of the build. |
| `multi-tenancy` | off | `with-db` | Explicit tenant scoping through `TenantEntity`, `TenantQueryExt`, and `TenantActiveModelExt`. See [Add row-level multi-tenancy](/docs/how-to/multi-tenancy). |
| `testing` | off | `dep:axum-test`, `dep:scraper`, `dep:tree-fs` | Test harness utilities. Enabled alongside `multi-tenancy` for docs.rs, and used by the crate's own `dev-dependencies`. |
| `cache_inmem` | **ON** | `dep:moka` | In-memory cache backend. |
| `cache_redis` | off | `dep:bb8-redis`, `dep:bb8` | Redis-backed cache pool. |
| `worker` | **ON** | `dep:sqlx`, `dep:ulid` | Background job queue/workers: the Postgres backend with `db-postgres`, the SQLite backend with `db-sqlite`. Which one runs is chosen at runtime by `queue.kind` in config (`Postgres` or `Sqlite`), not by a separate feature per database. |
| `worker_redis` | off | `worker`, `dep:redis` | Adds the Redis-backed queue backend on top of `worker` (implies it). Enable this if your app's `queue.kind` is `Redis`. |
| `redis_tls` | off | `redis/tokio-rustls-comp`, `redis/tls-rustls-webpki-roots`, `dep:rustls` | Redis over TLS (`rediss://` URLs) for managed providers such as ElastiCache, Upstash, or Azure Cache. Arms both the worker and the cache Redis paths at once — they share the same `redis` crate — with webpki-bundled roots, so it stays portable to slim/distroless images. Enable it alongside `worker_redis`/`cache_redis` and point the config at a `rediss://` URL; no code changes are needed. |
| `all_storage` | off | `storage_aws_s3` + `storage_azure` + `storage_gcp` | Umbrella flag — turns on every cloud storage backend at once. |
| `storage_aws_s3` | off | `opendal/services-s3` | AWS S3 storage backend. |
| `storage_azure` | off | `opendal/services-azblob` | Azure Blob storage backend. |
| `storage_gcp` | off | `opendal/services-gcs` | Google Cloud Storage backend. |
| `embedded_assets` | off | (empty — build-time flag) | Embeds the app's `assets/` directory into the compiled binary and swaps the view-engine's asset-loading path accordingly, instead of reading assets from disk at runtime. |

Source: root `Cargo.toml:41-109`.

## Interactions

- **`worker` unlocks the `jobs` subcommand.** `cargo loco jobs` (and its `cancel`/`tidy`/`purge`/`dump`/`import`/`requeue` subcommands) is compiled whenever the `worker` feature is enabled (`#[cfg(feature = "worker")]`, `src/cli.rs:27`). The same cfg gates the `JobStatus` import used by the jobs machinery. Since `worker_redis` implies `worker`, the `jobs` CLI is available for any queue backend — Redis, Postgres, or SQLite.
- **`debug_assertions` (not a Cargo feature) gates `generate` and `db entities`.** The `cargo loco generate` subcommand and the `db entities` subcommand are compiled only in debug builds (`#[cfg(debug_assertions)]`). They are unavailable in `--release` builds regardless of which Cargo features are on. `generate` additionally needs the `generators` feature; without it, it is an unrecognized subcommand.
- **Missing features fail loudly.** Without `db-postgres`, `queue.kind: Postgres` is an unknown variant when the config loads. Without `compression`, a `compression` middleware with `enable: true` is a boot error naming the feature. A disabled `compression` middleware needs no feature.
- **`all_storage` is a pure umbrella.** It has no dependency of its own; it just turns on `storage_aws_s3`, `storage_azure`, and `storage_gcp` together.
- **`multi-tenancy` implies `with-db`.** Tenant helpers build on Sea-ORM; enabling `with-db` alone does not enable them.
- **`auth` is pure Rust.** Loco signs and verifies its HMAC JWTs itself on the RustCrypto `hmac` and `sha2` crates, with a constant-time signature check, so JWT support needs no system C toolchain (e.g. OpenSSL) and pulls no RSA or elliptic-curve code.
- **`with-db` is a prerequisite, not an implication.** Enabling `worker` does not itself pull in `with-db`; the two are independent flags that happen to share the `sqlx` dependency.
- **The queue backend is chosen at runtime, among the ones compiled in.** `worker` builds in the Postgres queue provider with `db-postgres` and the SQLite one with `db-sqlite`; which one actually runs is decided by `queue.kind` (`Postgres` or `Sqlite`) in your app config. `worker_redis` adds the Redis provider, selected the same way with `queue.kind: Redis`. See [Choose a queue backend](/docs/how-to/choose-queue-backend).

## Disabling defaults

To opt out of the default set (e.g. a DB-less app), depend with `default-features = false` and re-list only the flags you want:

```toml
loco-rs = { version = "...", default-features = false, features = ["cli"] }
```

This is the pattern the `loco new` generator itself uses when the app is created without a database (see the CLI reference's app-creation flow): it emits `default-features = false` with `features = ["cli", "generators", "compression"]`, plus `worker_redis` if a Redis-backed queue was selected, `worker` and `db-postgres` if a Postgres-backed queue was selected, or `worker` and `db-sqlite` if a SQLite-backed queue was selected.

A SQLite-only app that wants the smallest build lists the drivers and codecs it uses instead of taking the defaults:

```toml
loco-rs = { version = "...", default-features = false, features = [
    "cli", "with-db", "db-sqlite", "worker", "cache_inmem", "compression", "generators",
] }
```

Dropping `generators` as well removes `loco-gen` from the build, at the cost of `cargo loco generate`.
