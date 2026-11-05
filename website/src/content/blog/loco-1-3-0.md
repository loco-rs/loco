---
title: "Loco 1.3.0"
description: "Faster builds for generated apps, SQLite you can run in production, server-side sessions and OAuth sign-in, and notifications."
pubDate: 2026-11-05T06:00:00Z
authors:
  - team-loco
---

1.3.0 is on crates.io. A fresh SQLite app with auth, workers and mail now does
31% less compiler work on a clean `cargo build`, and pulls 333 crates instead
of 460.

**Smaller builds.** Six dependencies are gone, replaced by small native code:
cron parsing, HS JWTs, the in-memory cache, byte sizes, `Cargo.lock` reading
and the local storage driver. Each replacement ran side by side with the crate
it replaced on a generated corpus before the crate went. `db-postgres`,
`compression` and `generators` are features now, so a generated app compiles
only its own database driver. `target/` drops from 2.7 GB to 1.9 GB, and the
no-database starter does 44% less work. The feature split started with
@cole-robertson.

**SQLite in production.** The PRAGMAs used to run on whichever pooled
connection came first, so the rest had no `busy_timeout` and no
`foreign_keys`. Every connection gets WAL, `synchronous=NORMAL` and a 5s
`busy_timeout` now. `cache.kind: Database` adds a cache in the app database,
and with the SQL queue a one-server app needs no Redis. The
[how-to](/docs/how-to/run-sqlite-in-production/) covers backups with Litestream.

**Sessions and OAuth sign-in.** `auth_session` adds signed cookies, a
`Session` and `CurrentUser<U>`. `auth_oauth` adds GitHub, Google and generic
providers with PKCE, and `cargo loco generate oauth github` wires it up. Set
`auth.session.store` to `database` or `redis` and the session lives on the
server, so `session.clear()` ends it for good. `sign_out_everywhere` ends all of a user's, and
`remember()` gives Devise-style remember me.

**Notifications.** A `Notifier` trait with Slack, signed-webhook and email
channels. `deliver_later` queues with retries and backoff. Webhooks are signed
with HMAC-SHA256, and `verify_signature` checks them on the receiving end.

**Storage from config.** A `storage:` block picks the driver, and `Aws` takes
an `endpoint` for MinIO or R2. From @robertazzopardi.

**Queues.** Unique jobs, retries with backoff, heartbeats, a real primary key,
a dequeue index, and opt-in `queue.retention` for old jobs.

## Fixed

S3, Azure and GCS requests all failed with "default HTTP transport is not
installed" after an OpenDAL bump. @mccormickt fixed it. `generate model`
without `sea-orm-cli` used to apply the migration and then fail, leaving you
to clean up by hand. It now checks first and writes nothing (@tomersein).
Enqueue on a managed Redis could time out at 1s after an idle spell, failing a
request that had already committed. It has real timeouts now (@mikosco4real).
Large numbers in job dumps keep their precision (@wheregmis). Native scheduler
jobs run without a shell (@damacus), and `cargo loco task` passes raw
arguments through (@pweaver).

## Upgrading

Bump `loco-rs` to `1.3`. With default features, most apps need nothing else.
If you set `default-features = false`, add `db-postgres`, `compression` or
`generators` if you use them, and `mailer` if you send mail. A missing one
fails at boot with a message naming it. Scheduler jobs with `shell: false` no
longer get shell expansion. The rest is in the
[changelog](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md).

Thanks to everyone who sent code this round.
