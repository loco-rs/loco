# Loco 0.17.0 — GitHub Finalization, Correspondence & Release-Notes Ledger

**Purpose.** Three roll-ready streams the user executes at publish time. I keep
this current as each epic lands. **Prep-only: I draft, the user posts/merges/publishes.**
Publish is **gated on Sea-ORM 2.0.0 stable** (currently `2.0.0-rc.41`).

Status legend: 🟢 done/adopted · 🟡 planned into an epic · 🔵 needs user decision · ⚪ untouched upstream

Last synced: 2026-07-02 (open PRs/issues snapshot). Re-sync before publish.

---

## Stream 1 — Correspondence ledger (per PR / issue)

For each item: **intended action**, **draft reply** (to post at publish), **credit**.
Nothing here is posted yet — GitHub is untouched.

### Open PRs

| PR | Author | Intended action | Notes |
|----|--------|-----------------|-------|
| #1698 Sea ORM 2.0 | elcoosp | 🟢 **Close as adopted** | Work incorporated via SeaQL-fork diff; credited in CHANGELOG + commit trailers. Draft reply below. |
| #1752 update bytes + jsonwebtoken (DRAFT) | AnthonyMichaelTDM | 🟡 Fold into Epic B security sweep | Security-relevant. Reconcile with dep upgrades. |
| #1772 bump actions/checkout 6→7 | dependabot | 🟢 **Adopted** (checkout v6→v7, all workflows) — close as done | Commit 3df22fc. |
| #1760 bump sccache-action | dependabot | 🟢 **Adopted** (v0.0.9→v0.0.10) — close as done | Commit 3df22fc. |
| #1757 bump rand 0.8→0.9 (loco-new) | dependabot | 🟢 **Adopted** (folded into version-skew unify) — close as done | Commit 787af3a. |
| #1754 bump yaml (docs-site npm) | dependabot | 🔵 **Merge directly on GitHub** | Transitive npm dep in docs-site pnpm-lock (yaml 2.5.0, parent-constrained). Not reconcilable offline without risking a half-written lock; dependabot regenerates the lock correctly on merge. Docs-site tooling only — orthogonal to the Rust release. |
| #1657 replace trait fn bounds w/ impl AsyncFn | alwayys-afk | 🟢 **Adopted** (commit 575f278) — close on release | Extended to all base_template test templates + snapshots. |
| #1762 migrate 8 READMEs → single NRG template | andriishin | 🔵 **Merge directly on GitHub** | Adds `nrg` generation tool + CI workflow (+922). Docs-infra, orthogonal to 0.17 correctness; deferred from Epic D by design (see spec 2026-07-03). |
| #1774 mailer implicit TLS (SMTPS/465) | zmilan | 🟢 **Adopted** (commit e1f4311) — close on release | Fixes #1773. |
| #1764 MultiEmail / multi-recipient | YtimoDeng | 🔵 Feature decision (not in the 5) | Overlaps #1694/#1652; revisit post-adoption. |
| #1694 Tera inheritance in emails | jtwaleson | 🔵 Feature decision (not in the 5) | Overlaps #1652. |
| #1693 Priority queue | jtwaleson | 🟢 **Adopted** (commit a685266) — close on release | Redis List→ZSET (breaking, documented), PG/SQLite priority column auto-migrated, `perform_later_with_priority`, mailer prio 100. 54 bgworker tests green. |
| #1742 scheduler/server without worker | mccormickt | 🟢 **Adopted** (commit 8b92b67) — close on release | Fixes #1737. |
| #1624 return job IDs from perform_later | NewtTheWolf | 🟢 **Adopted** (commit eae5ed2) — close on release | Fixes #1623. 48 bgworker tests green. |
| #1699 AWS Lambda deploy | SMCodesP | 🔵 Feature decision (not in the 5) | Large surface. |
| #1708 add popular tasks | floscodes | 🔵 Review | |
| #1685 PagerMeta on PageResponse | GoCoder7 | 🟢 **Adopted** (commit 20ee7bc) — close on release | Fixes #1683. |
| #1732 Vars::cli_arg → Result<&str> | dsgallups | 🔵 API-shape — reconcile in B | |
| #1730 Tasks API take [&String] (DRAFT) | pweaver | 🔵 API-shape — reconcile in B | |
| #1771 auto-formatting | D-system | 🔵 Review (may be superseded) | |
| #1770 fix i18n template error at boot | D-system | 🟡 Verify vs release/0.17.0 branch | Onboarding cluster. |
| #1758 new project generator fix | zjom | 🟡 Verify vs branch | Fixes #1749. |
| #1655 Vietnamese README | Nam-T | 🔵 Docs decision | |

### Open issues

| Issue | Intended action | Notes |
|-------|-----------------|-------|
| #1768 can't `cargo install loco` | 🟡 Verify fixed on branch → close in notes | Onboarding cluster. |
| #1749 generated project won't start | 🟡 Verify (PR #1758) | |
| #1759 SSR FTL bundle failure | 🟡 Verify | |
| #1770-related i18n boot | 🟡 Verify (PR #1770) | |
| #1755 generate controller not PK | 🟡 Verify vs bigint/gen changes | Touches modified schema/gen. |
| #1729 FK name mismatch create_table vs remove_reference | 🟡 Verify vs schema changes | |
| #1736 seeding from fixture fails on SQLite | 🟡 Verify | Sea-ORM 2.0 seed bounds changed. |
| #1691 more reliable seed dumping | 🔵 Feature decision | |
| #1766 Rails-style migrations | 🔵 Roadmap | |
| #1761 generate both api+template scaffold | 🔵 Feature (relates #1673) | |
| #1753 make internal components public | 🟡 Epic B (API/prelude curation) | |
| #1751 better error handling | 🟡 Epic B (#[non_exhaustive] Error) | |
| #1720 custom field types in entity gen | 🔵 Feature decision | |
| #1674 multi-layer caching | 🔵 Feature decision | |
| #1673 --service flag | 🔵 Feature decision | |
| #1640 multi-tenant/multi-app | 🔵 Roadmap | |
| #1714 contribute rhai-loco to org | 🔵 Org decision (user only) | |
| #1341 Redis TLS · #1191 Postgres TLS setup | 🔵 Verify/docs | |
| #1739 "Hello," (empty) | 🔵 Close as invalid | |

### Draft replies (post at publish; refine per final outcome)

**#1698 (close-as-adopted):**
> Thank you for kicking off the Sea-ORM 2.0 work — it was the seed for the
> 0.17.0 migration. The upgrade shipped in 0.17.0, built on the SeaQL fork's
> mechanical migration and your PR; both are credited in the changelog. Closing
> as adopted. 🙏

_(More per-item drafts added as each epic finalizes its outcome.)_

---

## Stream 2 — Release notes

**Vehicles:**
- `CHANGELOG.md` — Unreleased accumulates now; **final section restructure in Epic E**
  (Breaking Changes / Features / Fixes / Docs).
- `docs-site/content/docs/extras/upgrades.md` — 0.16→0.17 upgrade guide (live, appended per epic).
- Release announcement copy (headline) — assembled in Epic E.

**Breaking changes recorded so far (Epic A):**
1. Sea-ORM 2.0 + sqlx 0.9 + MSRV raise.
2. Generated primary/foreign keys now 64-bit (BIGINT / i64).

**To append as they land:** every Epic B dep/API break, Epic C LLM features,
Epic D doc changes. Rule: append to CHANGELOG + upgrade guide **at the moment**
the change is made, never reconstruct at the end.

---

## Stream 3 — Finalization runbook (user executes)

**Prep state (done, on branch `release/0.17.0`):** all epics complete; versions
bumped to 0.17.0 (loco-rs, loco-gen, loco `=loco-new`, `LOCO_VERSION`, loco-gen
path pin); CHANGELOG assembled; migration guide complete; green gate passing
(fmt, clippy, `cargo hack --each-feature` 18/18, `test --all-features` 597,
loco-gen 29). Everything below is for **you** — I do not push/tag/publish.

**Publish gate:** Sea-ORM **2.0.0 stable** on crates.io (today only
`2.0.0-rc.41` exists) with a `sea-schema` bound that fixes the 0.18.1 `?Send`
break (see migration-notes 2026-07-02). Until then, a *freshly generated* app
can't compile sea-orm; our committed lockfile (sea-schema 0.18.0) builds.

### Step 1 — When Sea-ORM 2.0.0 ships

```sh
# in the loco repo, on release/0.17.0
# flip rc → stable pins:
#   Cargo.toml:            sea-orm "2.0.0-rc" -> "2.0", sea-orm-migration "2.0.0-rc" -> "2.0"
#   loco-new/base_template/Cargo.toml.t + migration/Cargo.toml.t: same
#   src/doctor.rs:         MIN_SEAORMCLI_VER / min sea-orm "2.0.0-rc" -> "2.0"
# re-evaluate MSRV: workspace rust-version "1.94" -> "1.85" (the 2.0.0 target) if the stable tree allows.
cargo update -p sea-orm -p sea-orm-migration
cargo install sea-orm-cli --force        # 2.0 stable
```

### Step 2 — Full green gate on stable

```sh
export DOCKER_HOST="unix://$HOME/.colima/default/docker.sock"
cargo fmt --all --check
cargo clippy --workspace --all-features --tests -- -D warnings
cargo hack check --each-feature
cargo test --all-features
cargo test -p loco-gen                    # incl. test_migrations_flow (was upstream-blocked)
(cd loco-new && cargo test)               # wizard/matrix + generated-app boot (was upstream-blocked)
# sanity: loco new -> cargo build -> cargo loco start on a scratch app
```

### Step 3 — Publish (dependency order)

```sh
cargo publish -p loco-gen
cargo publish -p loco-rs
(cd loco-new && cargo publish)            # crate name: loco
```

### Step 4 — Tag + GitHub release

```sh
git push origin release/0.17.0            # then open/merge the release PR to master
git tag v0.17.0 && git push origin v0.17.0
gh release create v0.17.0 --notes-file <CHANGELOG 0.17.0 section>
```

### Step 5 — Community PRs (close with credit — all adopted locally)

Close as adopted, crediting the author (work is in 0.17.0):
**#1698** (Sea-ORM 2.0 seed), **#1685**, **#1742**, **#1774**, **#1624**,
**#1693**, **#1657**, dependabot **#1772/#1760/#1757**.
Merge directly on GitHub (not reconciled locally): **#1754** (docs-site npm
`yaml`), **#1762** (README `nrg` tooling — deferred docs-infra).

### Step 6 — Issues

- Close as fixed by 0.17.0 (verify on the stable build first): the onboarding
  cluster #1768/#1749/#1758/#1770/#1759, and #1755/#1729/#1736 (schema/gen)
  — confirm against the generated-app boot in Step 2.
- Fixed-by-adopted-PR: #1773 (→#1774), #1737 (→#1742), #1623 (→#1624),
  #1683 (→#1685).
- Triage remaining feature/roadmap issues (#1766, #1761, #1720, #1674, #1673,
  #1640, #1691, TLS #1341/#1191) into the 0.18 milestone; close #1739 (empty).

### Step 7 — Post correspondence

Post the Stream 1 draft replies as each PR/issue is closed.

### Deferred to 0.18 (documented in the design spec)

bgworker backend dedup · inflection consolidation · `db.rs`/`config.rs` splits ·
edition 2024 · snipdoc-blocking flip + orphaned-snippet restoration + generated
`llms.txt` pipeline · IA docs restructure · the non-"big-5" feature PRs
(#1764/#1694/#1693-adjacent/#1699/#1708) · `Error` narrowing.
