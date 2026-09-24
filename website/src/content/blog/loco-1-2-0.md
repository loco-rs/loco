---
title: "Loco 1.2.0"
description: "A Loco skill for coding agents, presigned storage URLs, row-level multi-tenancy, Tera components and Postgres-only builds. Plus a long list of fixes to the app loco new generates."
pubDate: 2026-09-24T06:00:00Z
authors:
  - team-loco
---

1.2.0 is on crates.io. Most of it came from people building real apps on 1.1
and sending PRs, which is my favorite kind of release.

**A Loco skill for coding agents.** A lot of Loco code gets written by an agent
now, and agents guess. They reach for a crate Loco already ships, hand-wire a
queue, or invent an API name that sounds right. `loco new` now writes a skill
into every app under `.claude/skills/loco/`: the framework's doctrine, ten task
recipes, and an API index for `loco-rs` and Sea-ORM generated from rustdoc. It
matches the crate version your app compiles against, and you can read it at
[loco.rs/skills/loco](https://loco.rs/skills/loco/SKILL.md).

We wrote up how we built it, including the parts that didn't work:

1. [Rails had twenty years of training data. Loco has a skill.](https://loco.rs/blog/agents-1-no-training-data/)
2. [What's in the Loco skill](https://loco.rs/blog/agents-2-whats-in-the-skill/)
3. [Writing the doctrine down](https://loco.rs/blog/agents-3-writing-the-doctrine-down/)
4. [Ten eval runs and an honest null](https://loco.rs/blog/agents-4-ten-eval-runs/)
5. [The eval kept finding bugs in Loco](https://loco.rs/blog/agents-5-the-eval-found-loco-bugs/)

**Presigned storage URLs.** `ctx.storage.presign_get` and `presign_put` give a
client a time-limited URL that talks to S3, Azure or GCS directly, so a large
upload skips your app. From @robertazzopardi.

**Row-level multi-tenancy.** Turn on the `multi-tenancy` feature, mark an
entity's tenant column with `TenantEntity`, and scope queries with
`.in_tenant(id)`. `set_tenant` fills the key on new records and refuses to move
a record to another tenant. Scoping stays explicit, so a query without
`.in_tenant` is unscoped. From @shuvroroy. The
[guide](/docs/how-to/multi-tenancy/) walks through it.

**Tera components.** `TeraView::render_component` renders one Tera 2 component
by name. @schungx also wrote a guide to moving Tera 1 templates to Tera 2 with
an AI assistant, including the spots where it gets things subtly wrong.

**Postgres-only builds.** SQLite is its own `db-sqlite` feature now, on by
default. Leave it out and you skip `libsqlite3-sys` and its C toolchain. Also
from @robertazzopardi.

**Batch enqueueing.** `perform_all_later` enqueues a list of jobs in one round
trip, atomically, on every backend. From @mccormickt.

## Fixed

The generated app got the most attention. A fresh server-side app answered 404
at `/`. The SPA had no sign-up page, a dev proxy stuck on port 5150, and a build
that never typechecked. Scaffolded `create` and `delete` answered `GET`. The
generated model test asserted nothing, and the Dockerfile pinned a Rust older
than our MSRV. All fixed, with tests that fail without the fix.

A Redis worker could also quit on a slow or dropped connection while the
process stayed up. It now retries until shutdown. Thanks @mikosco4real for
tracking that one down.

Full list in the [changelog](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md).

## Upgrading

Bump `loco-rs` to `1.2`. Most apps need nothing else. Two changes might touch
you:

- `perform_later_with_priority` returns `Error::QueueProviderMissing` when no
  queue is configured. It used to log and hand back a made-up job id.
- `ExprTrait` left the prelude. It attached itself to every value, so
  `n.max(1)` stopped compiling. Import it by name if you use it.

Thanks to everyone who sent code this round, and keep the PRs coming.
