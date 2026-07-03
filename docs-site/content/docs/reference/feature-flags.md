+++
title = "Feature flags"
description = "The complete loco-rs Cargo feature matrix: defaults, what each flag enables, and how flags interact."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

`loco-rs` gates most of its optional functionality behind Cargo features, declared in root `Cargo.toml:27-64`. This page is the exhaustive matrix — every flag, its default state, what it turns on, and how flags interact with each other and with `cargo loco`.

## Defaults

```toml
default = ["auth_jwt", "cli", "with-db", "cache_inmem", "bg_redis", "bg_pg", "bg_sqlt"]
```

A plain `loco-rs = "..."` dependency (no `default-features = false`) pulls in JWT auth, the `cargo loco` CLI, Sea-ORM database support, the in-memory cache, and all three background-worker backends (Redis, Postgres, SQLite).

## The matrix

| Flag | Default | Enables (deps / sub-features) | Purpose |
|---|---|---|---|
| `auth_jwt` | **ON** | `dep:jsonwebtoken`, `jsonwebtoken/rust_crypto` | JWT authentication. Selects `jsonwebtoken`'s pure-Rust `rust_crypto` backend (jsonwebtoken 10 no longer bundles a crypto backend by default), so the flag stays self-contained and needs no C toolchain, even when enabled alone with `default-features = false`. |
| `cli` | **ON** | `dep:clap` | Enables the `cargo loco` runtime CLI (`src/cli.rs`). |
| `with-db` | **ON** | `dep:sea-orm`, `dep:sea-orm-migration`, `dep:sqlx`, `loco-gen/with-db` | Sea-ORM 2.0.0-rc database support. Gates the `db` CLI subcommand and the DB-dependent generators (`model`, `migration`, `scaffold`). |
| `testing` | off | `dep:axum-test`, `dep:scraper`, `dep:tree-fs` | Test harness utilities. Also the feature set built for docs.rs (`[package.metadata.docs.rs] features = ["testing"]`, `Cargo.toml:211-212`) and used by the crate's own `dev-dependencies`. |
| `cache_inmem` | **ON** | `dep:moka` | In-memory cache backend. |
| `cache_redis` | off | `dep:bb8-redis`, `dep:bb8` | Redis-backed cache pool. |
| `bg_redis` | **ON** | `dep:redis`, `dep:ulid` | Redis-backed background job queue/workers. |
| `bg_pg` | **ON** | `dep:sqlx`, `dep:ulid` | Postgres-backed background job queue/workers. |
| `bg_sqlt` | **ON** | `dep:sqlx`, `dep:ulid` | SQLite-backed background job queue/workers. |
| `all_storage` | off | `storage_aws_s3` + `storage_azure` + `storage_gcp` | Umbrella flag — turns on every cloud storage backend at once. |
| `storage_aws_s3` | off | `opendal/services-s3` | AWS S3 storage backend. |
| `storage_azure` | off | `opendal/services-azblob` | Azure Blob storage backend. |
| `storage_gcp` | off | `opendal/services-gcs` | Google Cloud Storage backend. |
| `integration_test` | off | (empty — no deps) | Test-gating marker only; carries no dependencies of its own. |
| `embedded_assets` | off | (empty — build-time flag) | Embeds the app's `assets/` directory into the compiled binary and swaps the view-engine's asset-loading path accordingly, instead of reading assets from disk at runtime. |

Source: root `Cargo.toml:27-64`.

## Interactions

- **`bg_redis` / `bg_pg` / `bg_sqlt` unlock the `jobs` subcommand.** `cargo loco jobs` (and its `cancel`/`tidy`/`purge`/`dump`/`import`/`requeue` subcommands) is compiled only when at least one of the three background-worker flags is enabled (`#[cfg(any(feature = "bg_redis", feature = "bg_pg", feature = "bg_sqlt"))]`, `src/cli.rs:27`). The same cfg gates the `JobStatus` import used by the jobs machinery.
- **`debug_assertions` (not a Cargo feature) gates `generate` and `db entities`.** The `cargo loco generate` subcommand and the `db entities` subcommand are compiled only in debug builds (`#[cfg(debug_assertions)]`, `src/cli.rs:29, 140, 173`). They are unavailable in `--release` builds regardless of which Cargo features are on.
- **`all_storage` is a pure umbrella.** It has no dependency of its own; it just turns on `storage_aws_s3`, `storage_azure`, and `storage_gcp` together.
- **`auth_jwt` selects `jsonwebtoken/rust_crypto`.** Because jsonwebtoken 10 unbundled its crypto backend, `auth_jwt` explicitly enables the `rust_crypto` sub-feature so JWT support keeps working without requiring a system C toolchain (e.g. OpenSSL).
- **`with-db` is a prerequisite, not an implication.** Enabling `bg_pg`/`bg_sqlt` does not itself pull in `with-db`; the two are independent flags that happen to share the `sqlx` dependency.

## Disabling defaults

To opt out of the default set (e.g. a DB-less app), depend with `default-features = false` and re-list only the flags you want:

```toml
loco-rs = { version = "...", default-features = false, features = ["cli"] }
```

This is the pattern the `loco new` generator itself uses when the app is created without a database (see the CLI reference's app-creation flow): it emits `default-features = false` with `features = ["cli"]`, plus `bg_redis` if a Redis-backed queue was selected.
