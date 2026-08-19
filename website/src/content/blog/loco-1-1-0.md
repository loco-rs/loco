---
title: "Loco 1.1.0"
description: "Tera 2, an AWS Lambda deployment target, --no-auth on scaffold, and cargo loco jobs retry. Plus a paste-in agent prompt that handles the upgrade for you."
pubDate: 2026-08-19
authors:
  - team-loco
---

1.0 went out a few weeks ago and people started actually building on it. That's
the useful part — you find out fast which of your ideas survive contact with
real apps. 1.1.0 is on crates.io now.

**Tera 2.** Views and mailers render on Tera 2. For most apps this is one line:
bump `fluent-templates` from `0.13` to `0.15`. That's the crate behind `t()` for
i18n, and it was the thing pinning Tera 1. Your templates don't need rewriting
unless they use `{% macro %}`, `{% import %}` or `v.0` — Loco's generated ones
use none of those. Two things to watch: custom filters and functions need new
signatures, and an undefined variable is now an error instead of rendering
empty. That last one catches mailers too, so a template referencing a field
that's sometimes missing will fail at send time. Add `| default(value="")`.

**Deploy to Lambda.** `cargo loco generate deployment lambda`. Loco's router is
a `tower::Service` and so is the Lambda runtime, so your app runs unchanged. We
delegate to `cargo-lambda` instead of dragging in an AWS SDK. HTTP only —
workers and the scheduler don't fit that model.

**`--no-auth` on scaffold.** Scaffolded routes take a JWT extractor on all five
handlers. Right default, but there was no way out of it and nothing said so, so
the first `curl` against a fresh scaffold answered 401 with no explanation. Our
own tutorial examples were among the casualties. `generate controller` gets the
mirror image — public by default, `--auth` to opt in.

**`cargo loco jobs retry`.** A failed job used to be terminal. `requeue` sounds
like the recourse but only rescues jobs a crashed worker stranded
mid-processing. Storage also grew `exists`, `list` and `stat`.

## Fixed

Two queue fixes, both now covered by tests that fail without them:

- **SQLite:** with `queue.dangerously_flush: true`, `clear` could leave the
  queue unable to hand out jobs.
  ([details](https://github.com/loco-rs/loco/releases/tag/v1.1.0))
- **Redis:** completed and failed jobs were invisible to `jobs dump`,
  `jobs purge` and the other operator commands.
  ([details](https://github.com/loco-rs/loco/releases/tag/v1.1.0))

Full list in the [changelog](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md).

## Upgrading

The [upgrade guide](/docs/extras/upgrades/) has a prompt you can paste straight
into Claude Code or any coding agent — it covers the whole 1.0 → 1.1 change
surface and applies only what your app actually uses.

By hand it's: bump `fluent-templates` to `0.15`, and if you scaffolded list
endpoints the JSON envelope changed (`per_page` → `page_size`, `total` →
`total_items`, plus a new `total_pages`) so the framework speaks one pagination
vocabulary now.

Thanks to everyone who filed issues and PRs through the first weeks of 1.0.
Keep them coming.
