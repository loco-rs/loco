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
| #1657 replace trait fn bounds w/ impl AsyncFn | alwayys-afk | 🟡 Epic B (API modernization) | On the B list already. |
| #1762 migrate 8 READMEs → single NRG template | andriishin | 🟡 Epic D (README consolidation) | |
| #1774 mailer implicit TLS (SMTPS/465) | zmilan | 🟡 **Adopt in 0.17.0** (credit) | Fixes #1773. Decision 2026-07-02: adopt all 5 feature PRs. |
| #1764 MultiEmail / multi-recipient | YtimoDeng | 🔵 Feature decision (not in the 5) | Overlaps #1694/#1652; revisit post-adoption. |
| #1694 Tera inheritance in emails | jtwaleson | 🔵 Feature decision (not in the 5) | Overlaps #1652. |
| #1693 Priority queue | jtwaleson | 🟡 **Adopt in 0.17.0** (credit) | Reconcile vs bgworker dedup. |
| #1742 scheduler/server without worker | mccormickt | 🟡 **Adopt in 0.17.0** (credit) | Fixes #1737. |
| #1624 return job IDs from perform_later | NewtTheWolf | 🟡 **Adopt in 0.17.0** (credit) | Fixes #1623. |
| #1699 AWS Lambda deploy | SMCodesP | 🔵 Feature decision (not in the 5) | Large surface. |
| #1708 add popular tasks | floscodes | 🔵 Review | |
| #1685 PagerMeta on PageResponse | GoCoder7 | 🟡 **Adopt in 0.17.0** (credit) | Fixes #1683. |
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

**Gate:** Sea-ORM 2.0.0 stable published. Confirm sea-schema bound fixed
(see migration-notes 2026-07-02 blocker). Then:

1. **Finalize versions** — bump workspace crates to 0.17.0; resolve MSRV to the
   stable number; drop the rc pins.
2. **Green gate** — full matrix + wizard build + generated-app boot on stable.
3. **Publish order (dependency-ordered):** `loco-gen` → `loco-rs` → `loco-new`.
4. **Merge/close PRs** — per Stream 1 table (merge folded PRs or close-as-adopted with credit).
5. **Close issues** — verified-fixed cluster closed referencing the release;
   declined items closed with rationale; roadmap items labeled.
6. **Tag + release** — git tag, GitHub release using Stream 2 announcement copy.
7. **Post correspondence** — Stream 1 draft replies.

Detailed per-item ordering finalized in Epic E once all outcomes are known.
