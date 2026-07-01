# Loco Maintenance & Modernization — Program Design

Date: 2026-06-28
Owner: Dotan J. Nahum (with Claude / Opus 4.8)
Status: Approved (roadmap) — M1 in progress

## Context

Loco (`loco-rs` 0.16.4) is a Rails-like, all-in-one framework for Rust. After
~4 months idle (last commit 2026-03-04), the goal is to resume active
maintenance and modernize. Snapshot at program start:

- ~42k LOC Rust: `src` ~31k, `loco-gen` ~3.5k, `loco-new` ~4.5k,
  `loco-cli` (15-line stub), `xtask` ~0.6k, `tests` ~2.9k.
- Docs: Zola site (`docs-site/`), ~35k words; `snipdoc` syncs code snippets into
  Markdown but is currently `continue-on-error` in CI (drift allowed).
- 23 open PRs, 25 open issues.
- MSRV 1.70 / edition 2021; local toolchain 1.93.
- CI: `fmt` + `clippy` pedantic/nursery, `cargo hack check --each-feature`,
  build, `test --all-features` (Postgres service), plus `loco-gen`/`loco-new`
  matrices and a sanity job that scaffolds + builds an app.

### Key decisions (settled with user)

1. **Workstream #3 = "modernize in place"** — incremental, test-guarded
   refactors; **no public-API breaks**. Breaking 1.0 cleanup is explicitly out
   of scope for now.
2. **Sequence = all four milestones, in order** (M1 → M2 → M3 → M4).
3. **LLM support = skill/AGENTS.md + `llms.txt`/`llms-full.txt` + scaffold into
   new projects.** No MCP server.

### Guiding principles

- Green baseline is the safety net for every later change. Establish first.
- Every modernization change is behavior-preserving and lands as its own
  test-backed PR.
- One verified source of truth feeds both human docs and LLM artifacts — never
  write the framework's knowledge twice.
- Reduce risk before reducing line count.

## Milestones

Each milestone gets its own detailed spec → plan → implementation cycle.

### M1 — Green baseline & unblock users *(in progress)*

1. Establish reproducible green locally (build, fmt, clippy, `cargo hack
   check --each-feature`, `test --all-features` w/ Postgres+Redis); document the
   exact dev setup.
2. Reproduce & fix onboarding breakage: #1768 (`cargo install loco`), #1749
   (fresh project won't start).
3. Land security fix PR #1752 (`bytes` + `jsonwebtoken`).
4. Sweep safe dependabot/ready PRs (#1772, #1760, #1757, #1754, …).
5. Triage all open PRs + issues into merge-now / needs-work / close.
6. Cut patch release 0.16.5.

**Exit:** CI green, fresh `loco new` app builds *and* runs, security patched,
release shipped.

### M2 — Modernize in place *(no API breaks)*

- Library upgrades behind the suite (evaluate Sea-ORM 2.0 #1698 — likely
  breaking, may defer), modern Rust idioms (#1657 `impl AsyncFn`), edition 2024
  + MSRV bump, replace hand-rolled code with crates, LOC reduction — each its
  own behavior-preserving PR.
- Roadmap-fit feature PRs: scheduler-without-worker (#1742/#1737), priority
  queue (#1693), job IDs (#1624/#1623), mailer implicit TLS (#1774/#1773).

**Exit:** measurable LOC reduction + current deps, all green, public API intact.

### M3 — First-class LLM support

- One snipdoc-verified source of truth → generates `llms-full.txt` **and** a
  Loco skill / `AGENTS.md`.
- `loco new` scaffolds agent context (AGENTS.md + skill/cursor rules) into every
  generated app.
- The skill teaches the all-in-one model the right way: routing,
  models/migrations, controllers, workers, scheduler, mailers, tasks, config,
  testing.

**Exit:** a 1M-context model armed with the artifact builds a correct Loco app;
new projects ship agent context.

### M4 — Docs restructure & rewrite

- Restructure IA, rewrite for accuracy, make `snipdoc check` **blocking**. Docs
  and M3 LLM artifacts derive from the same verified source.

**Exit:** docs provably match code (enforced in CI), coherent structure.

## Out of scope (for now)

- Breaking API changes / 0.17 or 1.0 cleanup.
- Full from-scratch rewrite.
- MCP server for LLM support.
