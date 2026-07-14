# Loco 1.0.0 — Release Execution Plan (single source of truth)

> **This file supersedes** `docs/superpowers/specs/2026-07-01-github-finalization-and-correspondence.md`
> and `docs/superpowers/specs/2026-07-03-loco-0.18.0-deferred-items.md`.
> Those remain as historical rationale; **track and execute from here.**

**Goal:** Ship **Loco 1.0.0** — the big breaking release (Sea-ORM 2.0, generator
rebuild, React-SPA frontend, feature-flag cleanup, edition 2024, error narrowing)
— **on `sea-orm 2.0.0-rc.41`**, without waiting for a Sea-ORM stable release.

**Release branch:** `release/1.0.0`. Verified superset — it already contains every
commit from `release/0.17.0` (50) and `release/0.18.0` (64); it is 165 commits
ahead of `master` and `1.0.0..0.18.0` / `1.0.0..0.17.0` are both empty. **No
branch-folding is required.** The old branches are historical.

## Governance constraints (do not violate)

- **Local commits only.** Claude does version bumps, code, correspondence *drafts*,
  and local commits. **Jondot alone** pushes, tags, opens/merges PRs, publishes to
  crates.io, and posts on GitHub.
- Every commit ends with trailer `Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS`.
- Green gate (run from repo root, Colima up):
  ```sh
  export DOCKER_HOST="unix://$HOME/.colima/default/docker.sock"
  cargo fmt --all --check
  cargo clippy --workspace --all-features --tests -- -D warnings
  cargo hack check --each-feature
  cargo test --all-features
  cargo test -p loco-gen --all-features   # NOTE: --all-features — default features
                                          # skip the with-db template/model tests
                                          # (masked stale int→i64 snapshots; see A7)
  (cd loco-new && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
  ```

---

## 0. Decisions ledger

Status: ✅ confirmed · 🟡 recommended-default (proceeding unless overridden) · 🔵 needs Jondot

| # | Decision | Resolution | Status |
|---|----------|-----------|--------|
| D1 | Version | **1.0.0** — one combined breaking release | ✅ |
| D2 | Sea-ORM | **Ship on `=2.0.0-rc.41`** (exact pin), do not wait for stable | ✅ |
| D3 | #1764 MultiEmail + #1694 Tera email inheritance | **Adopted + committed** (`b01b04f2` #1694, `555af66e` #1764) | ✅ |
| D4 | #1732 `Vars::cli_arg -> Result<&str>` | **Adopted + committed** (`c948dcc4`) | ✅ |
| D5 | #1730 Tasks API `[&String]` | **Defer** (still a draft upstream) | 🟡 |
| D6 | #1699 AWS Lambda deploy | **Defer** (large surface, own release) | 🟡 |
| D7 | #1657 AsyncFn request helpers | **CONFIRMED: close as deferred, with credit** (E0282 in generated auth tests). Post the "deferred, not rejected" reply. | ✅ |
| D8 | #1714 donate `rhai-loco` to org | **CONFIRMED: accept into the org** | ✅ |
| D9 | #1655 Vietnamese README (non-English README policy) | **CONFIRMED: accept** (sets translated-README policy) | ✅ |
| D10 | #1762 NRG README-generation tooling | **CONFIRMED: defer** to post-1.0 docs-infra pass | ✅ |
| D11 | #1754 docs-site npm `yaml` bump | **CONFIRMED: merge on GitHub** (regenerates lockfile) | ✅ |

---

## PART A — Engineering (Claude; local commits)

### A1. Version bump 0.17.0 → 1.0.0 — ✅ DONE (`53915e9f`)
- [ ] `Cargo.toml:16` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `Cargo.toml:62` — `loco-gen = { version = "0.17.0", ... }` → `"1.0.0"`
- [ ] `loco-gen/Cargo.toml:3` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `loco-new/Cargo.toml:5` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `loco-new/src/lib.rs:12` — `LOCO_VERSION: &str = "0.17"` → `"1.0"`
- [ ] Grep sweep: `grep -rn '0\.17\.0' --include=*.toml --include=*.rs --include=*.t .` → no stray release-version refs remain (ignore CHANGELOG history entries)
- [ ] Rename `## Unreleased` in `CHANGELOG.md` → `## 1.0.0 - <date at publish>`; retitle the header from 0.17.0 to 1.0.0
- [ ] Commit: `chore(release)!: bump workspace to 1.0.0`

### A2. Exact-pin Sea-ORM to rc.41 (reproducible fresh-app builds) — ✅ DONE (`53915e9f`, `8aea4165`, `4293aa95`)
- [ ] `Cargo.toml:68` — `sea-orm = { version = "2.0.0-rc", ... }` → `"=2.0.0-rc.41"`
- [ ] `Cargo.toml:196` — `sea-orm-migration version = "2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] `loco-new/base_template/Cargo.toml.t:35` — `sea-orm ... "2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] `loco-new/base_template/migration/Cargo.toml.t:16` — `"2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] Keep `sea-schema = "=0.18.0"` pin in the template (comment already explains the `?Send` break).
- [ ] `src/doctor.rs:39,52` — **leave as `2.0.0-rc`**: these are *minimum-version floors* for the user's installed `sea-orm-cli`, not build pins; an exact pin would wrongly reject rc.42+. (No change; noted so it isn't "fixed" by mistake.)
- [ ] `cargo update -p sea-orm -p sea-orm-migration` → confirm lockfile resolves to rc.41; commit updated `Cargo.lock`s (examples/demo, examples/reference_spa, loco-new).
- [ ] Commit: `chore(deps): exact-pin sea-orm 2.0.0-rc.41 for reproducible fresh-app builds`

### A3. Prove a freshly generated app compiles + boots on rc.41 (kills the last "unverified" caveat) — ✅ DONE
- [ ] `cd loco-new && LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco cargo run -- new --path /tmp/loco-smoke --name smoke --db sqlite --bg async --assets serverside` (non-interactive)
- [ ] In the generated app: `cargo build` → compiles clean against `=2.0.0-rc.41`
- [ ] `cargo loco db migrate` then `cargo loco start` → boots; hit `/` → 200. Kill.
- [ ] Repeat for `--assets clientside --db sqlite --bg queue-sqlite` (flagship SPA + worker path).
- [ ] Record results in this file under "Verification log" below. No commit (scratch app); delete `/tmp/loco-smoke*`.

### A4. Full green gate on the release branch — ✅ DONE (final post-A5 re-run clean)

> **Final full gate re-run on the post-A5 tree (2026-07-14):** `cargo test
> --all-features` → lib **490 pass**, integration **84 pass** + **113 pass** (32
> ignored = DB/redis container tests), 0 failed; `cargo hack --each-feature` →
> **16/16** feature combos check clean; fmt + clippy `--all-features -D warnings`
> clean; loco-new wizard matrix **6/6**. Both gate halves exit 0. Part A closed.
- [ ] Run the green-gate block (top of file) — record pass counts in Verification log.
- [ ] Confirm previously-"upstream-gated" tests now run against rc.41: `loco-gen test_migrations_flow`, `loco-new` wizard matrix. If any genuinely cannot pass on rc (not just slow), note precisely why + whether it blocks 1.0.

### A5. Adopt the accepted feature PRs (per D3/D4) — ✅ DONE
- [x] #1694 — Tera template inheritance + shared templates. **Reconciled**: rebuilt
  `Template` onto a full Tera instance but sourced from `crate::tera::instance()`
  (keeps Loco's built-in filters — a bare `Tera::default()` would have regressed
  them); added `mail_template_with_shared`; scaffold generates `src/mailers/shared/`.
  `Template::new` is now fallible (BREAKING; no in-repo caller outside the mailer
  module). Removed the orphaned `crate::tera::render`. Commit `b01b04f2`
  (Co-authored-by Jouke Waleson).
- [x] #1764 — MultiEmail/`mail_multi`/`mail_template_multi` + `MultiMailerWorker`;
  `send()` refactor; `mail_template_multi` uses the fallible Template API. Commit
  `555af66e` (Co-authored-by YtimoDeng).
- [x] #1732 — `Vars::cli_arg -> Result<&str>`; swept base_template + demo +
  reference_spa consumers (`.clone()` → `.to_owned()`). Commit `c948dcc4`
  (Co-authored-by Daniel Gallups).
- [x] CHANGELOG: Added (multi-recipient, template inheritance/shared) + Breaking
  (`cli_arg`, `Template::new` fallible). Commit `b081cb32`.

### A6. Re-scored §8 issues → 4 built for 1.0.0 (2026-07-14) — ✅ DONE

Owner directive: re-evaluate the 10 "post-1.0" issues on **feature value** +
**build-confidence** only (scope/effort ignored). Researched each (issue thread +
codebase + framework prior art via subagents); 4 crossed the bar and were built,
green-gated, and committed. Reply drafts: `RELEASE-1.0.0-newwork-replies.md`.

- [x] **#1341 Redis TLS + #1191 Postgres TLS** (`3f773108`). PG TLS works via the
  URL (`sslmode=`, already-compiled rustls) with no flag; added defensive TLS on
  the worker-only sqlx pool. New `redis_tls` feature (arms queue+cache, webpki
  roots, pure-Rust `ring` — verified no `aws-lc` in tree). New how-to
  `docs-site/.../connect-over-tls.md`.
- [x] **#1691 typed `db::dump` + #1736 datetime bug** (`551e82fa`). Reproduced
  #1736 first (`Json("premature end of input")` on SQLite `CURRENT_TIMESTAMP`
  text), then fixed via RFC3339 normalization in `dump_tables`; added typed
  streaming `db::dump::<A>()` + `Hooks::dump` + `--dump` routing. Tests pass on
  SQLite **and** Postgres.
- [x] **#1753 logger internals** (`fb545ed9`). `logger::init_layer` /
  `init_env_filter` now `pub` (narrow scope; declined the broad ask + banner).
- [x] CHANGELOG updated (`48a181e3`); re-scored §8 + drafts (`063ec040`).
- [x] **Green gate re-run (full):** loco-rs `--all-features` **492 + 84 + 113,
  0 failed**; `cargo hack --each-feature` **17/17**; clippy/fmt clean; both
  examples compile; **fresh generated app migrates + compiles** (`test_migrations_flow`
  sqlite green after reinstalling the stale local `loco` CLI — see finding 8).
- **Stay post-1.0:** #1720 (SeaORM-2.0 entity-first may moot), #1674 (coherence
  scoping), #1640 (design spike), #1766 (low-value ColType cleanup). **Decline:**
  #1673 (contested/fat-model conflict), #1761 (superseded by SPA scaffold).

### A7. Verification block: 4 triage PRs + onboarding-cluster issues (2026-07-14) — ✅ DONE

Verified each open PR/issue against the branch (subagents). Verdicts + finished
reply drafts: `RELEASE-1.0.0-verification-replies.md`. Two needed code:

- [x] **#1755** (model/scaffold with no fields omits the `id` PK → non-compiling
  entity) — was a **real, still-open bug**. Fixed (`47d03916`): `id` unconditional
  in `model.t` + regression test. Credit @labike.
- [x] **#1771** (auto-formatting) — adopted the 5 still-applicable clippy fixes
  (`11ce376e`, credit @D-system); rest superseded by the rebuild.
- [x] **Verified fixed-in-1.0.0** (close w/ credit): #1758/#1749 + #1770/#1759
  (i18n `shared.ftl`, `0e6fe874`), #1768 (`cargo install`, rhai 1.25 `a640a97f`),
  #1729 (FK naming, `f9b87a68`+`5a44657c`).
- [x] **#1708** (popular tasks) — **defer** (injects an unexplained `groups` table
  + comments out a wizard assertion; root cause never found). Reply drafted.
- [x] **Gate-gap fix + stale-snapshot cleanup:** running `cargo test -p loco-gen
  **--all-features**` surfaced stale int→i64 snapshots + a stale unit test
  (`test_int_is_32_bit...`) left over from `116f3a92`; the release gate ran
  loco-gen with *default* features and masked them. Regenerated snapshots
  (reviewed: only `Integer`→`BigInteger` + id-line whitespace) and fixed the test
  (`47d03916`). **Green-gate command updated to loco-gen `--all-features`.**

**STALE-SNAPSHOT PATTERN (important — verify before publish).** Two release
changes updated templates/code but left snapshot tests unregenerated, and the
gate as-run masked both:
  1. **int→i64** (`116f3a92`): loco-gen migration/model/scaffold snapshots + unit
     test — fixed in `47d03916`.
  2. **A2 exact-pin** (`53915e9f`): loco-new `templates::db::test_cargo_toml`
     snapshots still said `sea-orm = "2.0.0-rc"` vs the pinned `=2.0.0-rc.41` —
     fixed in `b…` (loco-new snapshot commit).
  Both are now green. **Before publishing, run the FULL gate once more** (all
  crates, loco-gen `--all-features`) to catch any remaining drift.

**WIZARD-MATRIX ENV PREREQS (three non-code gotchas found this session, all
local build-env state, none a code defect):**
  1. Reinstall the CLI first: `cargo install --path loco-new --force` (a stale
     `loco` in PATH generates pre-A5 code → E0308).
  2. Export `LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco` (else generated apps
     can't resolve the unpublished `loco-rs = "^1.0"`).
  3. Clear the shared wizard target if a sqlite case fails on `libsqlite3-sys`
     `bindgen.rs` (`rm -rf $TMPDIR/loco-new-wizard-target`) — incremental-cache
     corruption of the C-bindings crate, DB-less cases pass, sqlite cases fail.
  With all three, a fresh sqlite+serverside app passes pedantic/nursery clippy
  with zero warnings (verified manually).

---

## PART B — Correspondence (Claude drafts; Jondot posts)

All work below is **on `release/1.0.0`**, so every "adopted" reply is truthful once
1.0.0 publishes. Draft replies live here; Jondot posts as each item is closed.

### B1. Adopted — close with credit
| PR | Author | Fixes | Draft reply |
|----|--------|-------|-------------|
| #1698 Sea-ORM 2.0 | elcoosp | — | "Thanks for kicking off the Sea-ORM 2.0 work — it seeded the 1.0.0 migration. Shipped in 1.0.0, built on the SeaQL fork's mechanical migration and your PR; both credited in the changelog. Closing as adopted. 🙏" |
| #1685 PagerMeta on PageResponse | GoCoder7 | #1683 | "Adopted in 1.0.0 (credited). Closes #1683. Thank you!" |
| #1742 scheduler/server without worker | mccormickt | #1737 | "Adopted in 1.0.0 (credited). Closes #1737. Thanks!" |
| #1774 mailer implicit TLS (SMTPS/465) | zmilan | #1773 | "Adopted in 1.0.0 (credited). Closes #1773. Thanks!" |
| #1624 return job IDs from perform_later | NewtTheWolf | #1623 | "Adopted in 1.0.0 (credited). Closes #1623. Thanks!" |
| #1693 priority queue | jtwaleson | — | "Adopted in 1.0.0 — Redis List→ZSET (breaking, documented), PG/SQLite priority column, `perform_later_with_priority`. Credited. Thanks!" |
| #1772 checkout v6→v7 | dependabot | — | close as done (adopted) |
| #1760 sccache-action | dependabot | — | close as done (adopted) |
| #1757 rand 0.8→0.9 | dependabot | — | close as done (adopted) |

### B2. Adopted-then-reverted — needs Jondot wording (D7)
- **#1657 (alwayys-afk)** — AsyncFn request helpers. We adopted it, but it makes the
  **generated auth test templates** fail to compile (E0282) on the shipping
  toolchain, so 1.0.0 reverts to generic `FnOnce -> Fut` to keep generated apps
  building. Draft: *"Thank you — this is the right direction. We hit a snag: with
  it, generated apps' auth tests fail type inference (E0282) on the current
  toolchain, so 1.0.0 keeps the generic bound to protect the generated-app path.
  Reopening/tracking for when inference improves. Not rejected — deferred."*
  → **Confirm you're OK closing-as-deferred with credit (vs keeping it open).**

### B3. Merge directly on GitHub (Jondot; not reconcilable offline)
- [ ] **#1754** docs-site npm `yaml` bump — click Merge; dependabot regenerates the lockfile.
- [ ] **#1762** NRG README tooling (andriishin) — **deferred** (D10); either merge as docs-infra or leave for a docs pass. Draft: "Great tooling — deferring to a post-1.0 docs-infra pass so it lands with the IA restructure. Thanks!"

### B4. Feature PRs — decisions
| PR | Author | Decision | Draft reply |
|----|--------|----------|-------------|
| #1764 MultiEmail | YtimoDeng | Adopt (A5) | "Adopting for 1.0.0, reconciled with #1694. Credited. Thanks!" |
| #1694 Tera email inheritance | jtwaleson | Adopt (A5) | "Adopting for 1.0.0 alongside #1764. Credited. Thanks!" |
| #1732 cli_arg Result | dsgallups | Adopt (A5) | "Adopted in 1.0.0 (breaking, in the migration guide). Thanks!" |
| #1730 Tasks API [&String] | pweaver | Defer | "Looks promising — it's still a draft; let's finish it post-1.0. Keeping open." |
| #1699 AWS Lambda | SMCodesP | Defer | "Big, valuable surface — deferring to its own release so it gets proper docs/tests. Keeping open." |
| #1708 popular tasks | floscodes | **DEFER** (A7) | reply in verification-replies.md |
| #1771 auto-formatting | D-system | **PARTIAL-ADOPTED** (A7, `11ce376e`) | reply in verification-replies.md |
| #1770 i18n boot fix | D-system | **FIXED-in-1.0.0** (A7, `0e6fe874`) | close as superseded; reply drafted |
| #1758 new-project generator fix | zjom | **FIXED-in-1.0.0** (A7, `0e6fe874`) | close w/ credit; reply drafted |

### B5. Social / org (Jondot only)
- [ ] **#1714** donate `rhai-loco` → **accept** (D8). "Happy to bring it into the org — thank you."
- [ ] **#1655** Vietnamese README → **accept** (D9). Sets the non-English README policy.

### B6. Issues
- [ ] **Close as fixed/implemented in 1.0.0 (built this session, A6 — drafts in `RELEASE-1.0.0-newwork-replies.md`):** #1191, #1341 (TLS); #1691 + #1736 (typed `db::dump` + datetime fix); #1753 (logger pub).
- [x] **Close as fixed by 1.0.0 — VERIFIED (A7, drafts in `RELEASE-1.0.0-verification-replies.md`):** #1768 (rhai 1.25), #1749 + #1759 + #1770 (i18n `shared.ftl`), #1729 (FK naming), #1755 (field-less-model id PK — was a real bug, fixed `47d03916`).
- [ ] **Close as fixed-by-adopted-PR:** #1773→#1774, #1737→#1742, #1623→#1624, #1683→#1685.
- [ ] **Close as addressed:** #1751 (better error handling → `#[non_exhaustive]` Error + error narrowing shipped in 1.0.0).
- [ ] **Triage to a post-1.0 milestone:** #1766 Rails migrations, #1720 custom field types, #1674 multi-layer cache, #1640 multi-tenant. **Decline (evidenced):** #1673 `--service` flag (contested/fat-model conflict), #1761 api+template scaffold (superseded by SPA scaffold).
- [ ] **Close as invalid:** #1739 (empty "Hello,").

---

## PART C — Publish runbook (Jondot executes)

1. `git push origin release/1.0.0`; open the release PR → merge to `master`.
2. Publish in dependency order:
   ```sh
   cargo publish -p loco-gen
   cargo publish -p loco-rs
   (cd loco-new && cargo publish)   # crate name: loco
   ```
3. Tag + release:
   ```sh
   git tag v1.0.0 && git push origin v1.0.0
   gh release create v1.0.0 --notes-file <CHANGELOG 1.0.0 section>
   ```
4. Post B1–B6 replies; close the adopted PRs/issues.
5. Publish the announcement (headline drafted separately).

---

## Verification log (Part A)

**Commits so far:** `5f286ed8` (fmt), `53915e9f` (1.0.0 bump + exact-pin),
`116f3a92` (generator fixes), `8aea4165` (loco-new lock). **A5:** `c948dcc4`
(#1732 cli_arg), `b01b04f2` (#1694 mailer inheritance), `555af66e` (#1764
MultiEmail), `b081cb32` (CHANGELOG + drop dead `tera::render`), `4293aa95`
(fmt column.rs + refresh demo/reference_spa lockfiles to 1.0.0).

### A5 verification (feature-PR adoption)

- ✅ **All three PRs adopted, reconciled, committed with author credit.** Key
  reconciliation: #1694 was written against a bare `Tera::default()`; this branch
  renders mailer templates *with* app filters via `tera::instance()`, so the
  reworked `Template` builds from `instance()` to preserve filter access (verified:
  `©` passes through un-escaped, filter-free fixtures render identically).
- ✅ **loco-rs `--all-features` lib: 490 pass, 0 fail** (was 484; +6 new mailer
  tests: 3 template inheritance/shared + 3 multi-recipient). All snapshots
  regenerated + reviewed (inheritance renders base layout + filled blocks; multi
  renders multi-To header, envelope-only BCC, visible CC).
- ✅ **loco-gen mod: 10 pass** — mailer scaffold snapshots regenerated + reviewed
  (`{% raw %}` wrappers produce literal `{% extends %}`/`{% block %}`).
- ✅ **fmt clean; clippy `--all-features --tests -D warnings` clean** (loco-rs +
  loco-gen); loco-new fmt + clippy clean.
- ✅ **End-to-end generator guarantee:** ran `cargo loco generate mailer notify`
  in `examples/reference_spa` (local loco-gen + local loco-rs path deps) → real
  generated `notify.rs` uses `mail_template_with_shared` + shared/welcome dirs →
  **`cargo check` compiled clean** (exit 0). Scaffold reverted; tree clean.
- ✅ **Hygiene:** committed HEAD's `examples/demo` + `examples/reference_spa`
  lockfiles still pinned loco-rs 0.17.0 (A2 gap); refreshed both to 1.0.0.
- ✅ **loco-new wizard matrix: 6/6 combos pass, 0 fail** (`test_starter_combinations`,
  ~21.5 min, exit 0). Ran end-to-end against `LOCO_DEV_MODE_PATH` (local loco-rs +
  loco-gen). Each combo generates a full app, runs clippy `-D warnings` + its test
  suite (32 pass each), then generates controller/task/scheduler/worker/**mailer**/
  scaffold/model/**`age:int` migrations** and recompiles. This DOES exercise the A5
  #1694 mailer scaffold (`src/mailers/shared/base.t` + `mail_template_with_shared`
  generated and compiled) and the int→bigint fix. Zero `error[E…]`, zero
  "could not compile" across the full 5.8k-line log.

- ✅ **A1** version bump 0.17.0 → 1.0.0 (5 sites) + CHANGELOG merged into one `## 1.0.0`.
- ✅ **A2** exact-pin `sea-orm =2.0.0-rc.41` (4 sites); lockfiles regenerated.
- ✅ **A3** fresh serverside/sqlite app: compiles on rc.41, `db migrate` applies,
  boots — `/_health` 200, `/_ping` 200.
- ✅ **A4 (partial)** fmt clean; clippy `--all-features -D warnings` clean;
  `cargo hack --each-feature` exit 0; loco-gen lib 36 tests pass.
- ✅ **test_migrations_flow sqlite**: full flow runs end-to-end on rc.41; snapshot
  regenerated + verified + accepted.
- ✅ **test_migrations_flow postgres** (Postgres.app, PG 17.5): full flow runs
  end-to-end on rc.41; snapshot regenerated + verified (int→bigint,
  small_unsigned→smallint, array_int→bigint[], json_uniq gone, jsonb_uniq kept)
  + accepted. Needs a pre-created DB (`createdb loco_mig_test`) + `DATABASE_URL`.
- ✅ **A4** `cargo test --all-features` (loco-rs): lib **484 pass**; integration had
  2 pre-existing flaky JWT failures (see finding 7) — fixed, re-confirmed green.
  (superseded — the wizard matrix later completed 6/6 green end-to-end; see the
  A5 verification block above.)

### Findings surfaced while un-gating test_migrations_flow (all real, all fixed)

Un-gating the exhaustive migration test — invisible to the wizard-matrix tests,
which only scaffold string/reference columns — caught **six** real bugs:

1. **Exact-pin was a required correctness fix, not hygiene.** rc.42 shipped; the
   old loose `2.0.0-rc` caret drifted a fresh `loco new` to rc.42, which collides
   with the pinned `sea-schema =0.18.0` (`?Send` break) → every fresh app failed
   to compile. `=2.0.0-rc.41` fixes it. (`53915e9f`)
2. **`int` was broken on SQLite** — 32-bit DTO vs i64 entity → non-compiling
   scaffold. Now 64-bit. **DECISION (surface):** `int` ≡ `big_int` now (both i64);
   there is no 32-bit scaffold DSL type. Forced by SQLite + matches CHANGELOG.
   Overridable. (`116f3a92`)
3. **`decimal_len!:8:24` didn't parse** (flag-after-base-name). Fixed. (`116f3a92`)
4. **`json^` (unique json) failed at migrate on Postgres** (no btree opclass,
   SQLSTATE 42704). Now rejected at generation with a "use jsonb" message; jsonb^
   stays valid. (`c70724cf`)
5. **`small_unsigned` didn't compile on Postgres** (sea-orm `SmallUnsigned`
   round-trips i16 on SQLite but i32 on PG). Now a signed `SmallInteger` (i16),
   matching the DTO + existing SQLite behavior. (`c70724cf`)
6. (arrays) `array:int` follows int→64-bit (`bigint[]`/`Vec<i64>`), verified on PG.
7. **2 flaky JWT expiration tests** (`--all-features` integration) minted
   `generate_token(0)` (exp == now) and asserted 401. jsonwebtoken 10.4.0 treats
   a token as valid *through* its exp second (`now > exp`, leeway=0), so exp==now
   is second-boundary flaky. Not a security issue (a token expired ≥1s is
   rejected; `can_handle_expired_jwt_token` already covers it robustly). Removed
   the two redundant tests. Not my regression. (`<jwt commit>`)
8. **`test_migrations_flow` failed on a stale local CLI (2026-07-14, A6).** The
   test drives the *installed* `loco` binary; the one in PATH was dated Jul 12 —
   a day before A5's `cli_arg` base_template `.to_owned()` fix (`c948dcc4`,
   Jul 13) — so it generated pre-A5 `.clone()` code → 3× `E0308` in
   `tasks/user_create.rs`. **Not a code regression** (both examples compile; my
   A6 commits never touch that area). Fixed by `cargo install --path loco-new
   --force`; re-ran → **green** (1 passed, exit 0). Reminder for the publish
   env: reinstall the `loco` CLI before running generator tests.

**Type-portability note (context for the `int`/`small_unsigned` decisions):** only
`smallint`(i16) and `bigint`(i64) introspect consistently across SQLite+Postgres;
32-bit `integer` does not (SQLite widens to i64, PG keeps i32), and neither DB has
native unsigned ints. So the portable scaffold DSL now resolves to: small_int/
small_unsigned→i16, int/big_int/unsigned/big_unsigned→i64. This is a deliberate,
overridable simplification.
