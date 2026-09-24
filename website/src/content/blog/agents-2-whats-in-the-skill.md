---
title: "What's in the Loco skill"
description: "llms.txt didn't reach anyone, a full dump was too big to reason over, and a generated API index turned out to be a hallucination detector. Part 2 of teaching agents Loco."
pubDate: 2026-09-24T02:00:00Z
authors:
  - dotan-nahum
---

Our first answer to "help agents write Loco" was `llms.txt`. We had one. It was
a list of links to docs, and nothing fetches it unless someone points an agent
at it. We also had `llms-full.txt`, which is every doc page in one file: 446KB,
around 110k tokens. That's the trap Svelte fell into when Svelte 5 shipped and
models kept writing Svelte 4. Their official small file was over 130k tokens,
and what people used instead was a
[hand-trimmed version](https://github.com/martypara/svelte5-llm-compact) of the
things a model gets wrong. Small and pointed beats complete.

[Convex](https://stack.convex.dev/convex-evals) was the other precedent. Their
curated guidelines lifted agent success by about 20%. A year later the gap had
shrunk to a few points, because Convex had landed in the training data. That
won't happen for Loco on any schedule we control, so whatever we ship has to
keep working.

## The shape

We went with an [Agent Skill](https://agentskills.io): a folder the agent
discovers, with a short entry file that routes to the rest on demand. `loco new`
writes it into every app at `.claude/skills/loco/`, pinned to the `loco-rs`
version that app compiles against. The same files live at
[loco.rs/skills/loco](https://loco.rs/skills/loco/).

- **`SKILL.md`**, under 2k tokens. Three rules (generate then edit, use the
  batteries, fat model slim controller), the eight fields of `AppContext`, the
  file layout, the CLI, and a table that says which file to read for what you're
  doing right now.
- **`doctrine.md`**. What good Loco code looks like, and the seven places Rust
  forced us away from Rails. It gets [its own post](/blog/agents-3-writing-the-doctrine-down/).
- **`recipes/`**. Ten task-shaped guides: endpoint, model and migration,
  background job, task and schedule, mailer, cache, config, middleware, auth,
  testing.
- **`errors.md`**. Symptom to cause, for the compiler errors that point away
  from their own cause. Sea-ORM's "no method named `order_by_desc`" is a trait
  not in scope, not a missing method.
- **`api-index.md`** and **`sea-orm-index.md`**. Every public symbol, grouped by
  module, generated from rustdoc JSON.

## The index is a lie detector

The index is the file we'd keep if we could keep one. An agent about to write
`use loco_rs::jobs::Job` can check first, and the answer is no. When we tested
it, it found all six real symbols we asked about and rejected all five
plausible fakes: `app::AppState`, `jobs::Job`, `controller::Controller`,
`mailer::send_mail`, `db::Pool`.

It also taught us not to trust ourselves. Its first version published rustdoc's
*definition* paths, including `model::query::paginate::PaginationQuery`. That
module is private. An agent read our anti-hallucination file and wrote a path
that doesn't compile. We now whitelist the real public paths and render trait
bounds too, because an index that drops the `where` clause tells an agent `F`
can be anything.

## The escape hatch

Agents will read framework source when they're stuck. We stopped fighting that
and wrote it into `SKILL.md`: the source is already on your disk under
`~/.cargo/registry/src`, grep one module, and do it last. An undocumented escape
hatch gets used first. A documented one gets used last.

## Keeping it true

A skill that drifts from the crate is worse than none. `cargo xtask agent-skill
--check` regenerates the indexes, the copy shipped by `loco new`, the root
`AGENTS.md` and the website copy, and fails CI if any of them differ from what's
committed. It fired on the 1.2.0 release PR: Sea-ORM shipped a patch, the index
was stale, and the build went red until we regenerated it.

Next: [writing down the doctrine](/blog/agents-3-writing-the-doctrine-down/).
