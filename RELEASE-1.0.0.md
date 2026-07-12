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
  cargo test -p loco-gen
  (cd loco-new && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
  ```

---

## 0. Decisions ledger

Status: ✅ confirmed · 🟡 recommended-default (proceeding unless overridden) · 🔵 needs Jondot

| # | Decision | Resolution | Status |
|---|----------|-----------|--------|
| D1 | Version | **1.0.0** — one combined breaking release | ✅ |
| D2 | Sea-ORM | **Ship on `=2.0.0-rc.41`** (exact pin), do not wait for stable | ✅ |
| D3 | #1764 MultiEmail + #1694 Tera email inheritance | **Adopt** (reconcile the two together) | 🟡 |
| D4 | #1732 `Vars::cli_arg -> Result<&str>` | **Adopt** (small breaking tidy) | 🟡 |
| D5 | #1730 Tasks API `[&String]` | **Defer** (still a draft upstream) | 🟡 |
| D6 | #1699 AWS Lambda deploy | **Defer** (large surface, own release) | 🟡 |
| D7 | #1657 AsyncFn request helpers | **Adopted then REVERTED** (broke generated auth tests, E0282). Reply = "deferred, not rejected." Confirm wording. | 🔵 |
| D8 | #1714 donate `rhai-loco` to org | **Accept** — Jondot's org call | 🔵 |
| D9 | #1655 Vietnamese README (non-English README policy) | **Accept** | 🔵 |
| D10 | #1762 NRG README-generation tooling | **Defer** (docs-infra, merge on GitHub later) | 🔵 |
| D11 | #1754 docs-site npm `yaml` bump | **Merge on GitHub** (regenerates lockfile) | 🔵 |

---

## PART A — Engineering (Claude; local commits)

### A1. Version bump 0.17.0 → 1.0.0
- [ ] `Cargo.toml:16` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `Cargo.toml:62` — `loco-gen = { version = "0.17.0", ... }` → `"1.0.0"`
- [ ] `loco-gen/Cargo.toml:3` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `loco-new/Cargo.toml:5` — `version = "0.17.0"` → `"1.0.0"`
- [ ] `loco-new/src/lib.rs:12` — `LOCO_VERSION: &str = "0.17"` → `"1.0"`
- [ ] Grep sweep: `grep -rn '0\.17\.0' --include=*.toml --include=*.rs --include=*.t .` → no stray release-version refs remain (ignore CHANGELOG history entries)
- [ ] Rename `## Unreleased` in `CHANGELOG.md` → `## 1.0.0 - <date at publish>`; retitle the header from 0.17.0 to 1.0.0
- [ ] Commit: `chore(release)!: bump workspace to 1.0.0`

### A2. Exact-pin Sea-ORM to rc.41 (reproducible fresh-app builds)
- [ ] `Cargo.toml:68` — `sea-orm = { version = "2.0.0-rc", ... }` → `"=2.0.0-rc.41"`
- [ ] `Cargo.toml:196` — `sea-orm-migration version = "2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] `loco-new/base_template/Cargo.toml.t:35` — `sea-orm ... "2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] `loco-new/base_template/migration/Cargo.toml.t:16` — `"2.0.0-rc"` → `"=2.0.0-rc.41"`
- [ ] Keep `sea-schema = "=0.18.0"` pin in the template (comment already explains the `?Send` break).
- [ ] `src/doctor.rs:39,52` — **leave as `2.0.0-rc`**: these are *minimum-version floors* for the user's installed `sea-orm-cli`, not build pins; an exact pin would wrongly reject rc.42+. (No change; noted so it isn't "fixed" by mistake.)
- [ ] `cargo update -p sea-orm -p sea-orm-migration` → confirm lockfile resolves to rc.41; commit updated `Cargo.lock`s (examples/demo, examples/reference_spa, loco-new).
- [ ] Commit: `chore(deps): exact-pin sea-orm 2.0.0-rc.41 for reproducible fresh-app builds`

### A3. Prove a freshly generated app compiles + boots on rc.41 (kills the last "unverified" caveat)
- [ ] `cd loco-new && LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco cargo run -- new --path /tmp/loco-smoke --name smoke --db sqlite --bg async --assets serverside` (non-interactive)
- [ ] In the generated app: `cargo build` → compiles clean against `=2.0.0-rc.41`
- [ ] `cargo loco db migrate` then `cargo loco start` → boots; hit `/` → 200. Kill.
- [ ] Repeat for `--assets clientside --db sqlite --bg queue-sqlite` (flagship SPA + worker path).
- [ ] Record results in this file under "Verification log" below. No commit (scratch app); delete `/tmp/loco-smoke*`.

### A4. Full green gate on the release branch
- [ ] Run the green-gate block (top of file) — record pass counts in Verification log.
- [ ] Confirm previously-"upstream-gated" tests now run against rc.41: `loco-gen test_migrations_flow`, `loco-new` wizard matrix. If any genuinely cannot pass on rc (not just slow), note precisely why + whether it blocks 1.0.

### A5. Adopt the accepted feature PRs (per D3/D4)
- [ ] #1764 + #1694 — reconcile multi-recipient + Tera email inheritance into one coherent mailer change; port tests; green. Commit crediting both authors (Co-authored-by trailers).
- [ ] #1732 — `Vars::cli_arg` returns `Result<&str>`; sweep call sites; CHANGELOG breaking note. Commit crediting dsgallups.
- [ ] Append CHANGELOG entries (Added: multi-recipient/email inheritance; Breaking: `cli_arg` signature).

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
| #1708 popular tasks | floscodes | Review | (triage — needs a look) |
| #1771 auto-formatting | D-system | Review | (may be superseded by generator changes — verify vs branch) |
| #1770 i18n boot fix | D-system | Verify vs 1.0.0 | if already fixed on branch: "Fixed on the 1.0.0 branch — thanks for the report/fix." |
| #1758 new-project generator fix | zjom | Verify vs 1.0.0 | fixes #1749; likely already covered by generator rebuild — verify then credit/close |

### B5. Social / org (Jondot only)
- [ ] **#1714** donate `rhai-loco` → **accept** (D8). "Happy to bring it into the org — thank you."
- [ ] **#1655** Vietnamese README → **accept** (D9). Sets the non-English README policy.

### B6. Issues
- [ ] **Close as fixed by 1.0.0** (verify against A3 generated-app boot first): onboarding cluster #1768, #1749, #1759, #1755, #1729, #1736; i18n #1770.
- [ ] **Close as fixed-by-adopted-PR:** #1773→#1774, #1737→#1742, #1623→#1624, #1683→#1685.
- [ ] **Close as addressed:** #1751 (better error handling → `#[non_exhaustive]` Error + error narrowing shipped in 1.0.0).
- [ ] **Triage to a post-1.0 milestone:** #1766 Rails migrations, #1761 api+template scaffold, #1720 custom field types, #1674 multi-layer cache, #1673 `--service` flag, #1640 multi-tenant, #1691 seed dumping, #1753 public internals, TLS #1341/#1191.
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

## Verification log (fill during Part A)

- A3 serverside/sqlite smoke: _pending_
- A3 clientside/queue-sqlite smoke: _pending_
- A4 green gate (fmt/clippy/hack/test/loco-gen/loco-new): _pending_
- A4 formerly-gated tests (test_migrations_flow, wizard matrix) on rc.41: _pending_
