# Loco 1.0.0 — Release-Day Playbook

> **Single operational runbook to ship 1.0.0 cleanly, in one day.**
> Companion to `RELEASE-1.0.0.md` (the decisions/engineering source of truth).
> This file is the *sequence to execute*; `RELEASE-1.0.0.md` is the *rationale*.
>
> **Governance model (non-negotiable):** Claude **prepares**; Jondot **presses**
> every irreversible button — push, PR merge, `cargo publish`, tag, GH release,
> posting replies, org transfers. Owner tags below: `[CLAUDE]` prep, `[JONDOT]`
> button, `[LOCAL]` needs Jondot's machine (Docker/Postgres/`LOCO_DEV_MODE_PATH`).

---

## Status at a glance

| Track | State |
|---|---|
| **Part A — Engineering** | ✅ 100% done. All 3 crates at 1.0.0; `sea-orm =2.0.0-rc.41`; **E0282 fixed on-branch** (`20518c62`) |
| **Part B — Correspondence** | ✍️ ~30 reply drafts written, **nothing posted** |
| **Part C — Publish runbook** | 🔒 prep-only, **not executed** |
| **Website** | Astro rebuild parked in worktree `feat/website-blog-casts` — **decoupled** from 1.0 (fast-follow) |

**Publishable crates & version:** `loco-gen`, `loco-rs`, `loco` (dir `loco-new`) — all `1.0.0`.
Versions are **not** workspace-inherited; keep them in lockstep by hand.
`xtask bump`'s `LOCO_VERSION` regex is stale (`"0.13"`) — verify `loco-new/src/lib.rs:12` = `"1.0"` manually.

---

## Confirmed decisions (locked)

1. **Decouple the website.** Ship framework 1.0.0 today on the **current** docs-site; cut the Astro site over as an immediate fast-follow.
2. **Close old-site docs PRs #1778/#1779/#1780** with thanks + credit, citing the ground-up rebuild (already fixes theme-flash/dark-mode).
3. **Ship on `sea-orm =2.0.0-rc.41`** (stable gate dropped). CHANGELOG/announcement must **not imply stable Sea-ORM** (fix the "MSRV 1.85 / 2.0.0 stable" wording; we ship rc.41 at MSRV 1.94).
4. **Land only the safe set** (#1732, #1624, #1742, dependabot #1772/#1760/#1754, #1655); defer/close everything with API-surface or scope risk (see PR disposition).
5. **First-run reports #1749/#1759/#1768 are a hard go/no-go** — reproduce-or-refute on the branch before tagging.

---

## Phase 0 — Consolidate  ·  *blocker*

- [ ] **Park the concurrent session on `release/1.0.0`.** HEAD moved `3ee66a21 → 20518c62` during investigation — a second driver is/was live. One driver only before any tree edits.  `[JONDOT]`

## Phase 1 — Content prep  ·  `[CLAUDE]` in the release tree

- [ ] **Migration guide** (`docs-site/content/docs/extras/upgrades.md`): relabel `0.16→0.17` → **0.x→1.0**; fix body text still saying "0.17.0" / "drain Redis before 0.17.0"; add the ~11 missing Breaking stanzas so it matches CHANGELOG (`MultiDbInitializer`, `AppContext` `#[non_exhaustive]`, storage `ReplicatedStrategy`, local-root security change, `Queue::empty()`, fallback 404, `{env}.local.yaml` deep-merge, corrected HTTP status codes, `JWT::algorithm()` HMAC-only, `trusted_proxies`/`remote_ip`, `TeraView::build_with_post_process`) + the A5 items (`cli_arg -> Result`, `Template::new` fallible, mailer-scaffold) — REMAINING-WORK #49.
- [ ] **CHANGELOG wording**: fix the "stable Sea-ORM / MSRV 1.85" phrasing to honest rc.41 framing. Leave `## 1.0.0` **undated** until Phase 4.
- [ ] **Announcement draft** (blog + Discord + X) — Claude drafts; Jondot owns final voice/channels. (REMAINING-WORK #52.)
- [ ] **#1657 reply**: finalize close-as-deferred-with-credit (D7 ✅).

## Phase 2 — Final verification  ·  `[LOCAL]`

- [ ] **Reproduce-or-refute #1749 / #1759 / #1768** on the branch: fresh `loco new` → build → boot; `cargo install --path loco-new`. Fix if real; else replies say "fixed in 1.0.0, please re-verify."  **← go/no-go**
- [ ] **Full green gate once more, with `loco-gen --all-features`** (mandatory stale-snapshot re-run — default features mask the with-db template/model tests).
- [ ] **`cargo publish --dry-run`** for `loco-gen`, `loco-rs`, `loco` — verify metadata + path/version deps resolve. (REMAINING-WORK #53.)
- [ ] Verify `loco-new/src/lib.rs` `LOCO_VERSION == "1.0"` by hand.

## Phase 3 — GitHub hygiene  ·  `[JONDOT]` presses (drafts ready)

- [ ] Create the **missing `1.0.0` milestone** + `blocker` / `1.0` labels (none exist today).
- [ ] Merge the safe set (decision 4). **Close #1698** (superseded Sea-ORM 2.0); close **#1778/#1779/#1780** (decision 2).
- [ ] Post drafted replies — B1–B6, `RELEASE-1.0.0-newwork-replies.md`, `RELEASE-1.0.0-verification-replies.md` — **after** publish so "fixed in 1.0.0" is true.
- [ ] Move deferred items to the post-1.0 milestone.

## Phase 4 — Publish  ·  `[JONDOT]`

- [ ] Stamp `## 1.0.0 - <publish date>`. Push `release/1.0.0` → release PR → merge to `master`.
- [ ] **Publish in dependency order:** `cargo publish -p loco-gen` → `cargo publish -p loco-rs` → `(cd loco-new && cargo publish)`.
- [ ] `git tag v1.0.0 && git push origin v1.0.0`; `gh release create v1.0.0 --notes-file <1.0.0 CHANGELOG section>`.
- [ ] Publish the announcement; post release-tied closes/credits.

## Phase 5 — Post-publish fast-follow  ·  same day

- [ ] Clean-machine smoke test: `cargo install loco` → `loco new` → boot (the real user path).
- [ ] **Website cutover**: merge the Astro worktree; port the completed migration guide into it.
- [ ] Backlog triage wave: label remaining ~20 PRs/issues to the post-1.0 milestone.

---

## Go / No-Go gate (all ✅ before Phase 4)

- [x] Engineering green (Part A)
- [ ] Full gate re-run with `loco-gen --all-features`  *(LOCAL — Jondot)*
- [~] #1749 / #1759 / #1768 refuted-or-fixed — **static: fixed in HEAD** (rhai 1.25 `a640a97f`; `shared.ftl` moved out of `assets/i18n/` `0e6fe874`; field-less PK `47d03916`); runtime confirm pending
- [ ] `cargo publish --dry-run` ×3 clean  *(LOCAL — Jondot)*
- [x] Migration guide 1.0-complete — `98e44707`
- [x] CHANGELOG wording honest (no "stable Sea-ORM") — `98e44707`
- [x] Announcement ready — `RELEASE-1.0.0-announcement.md`

---

## Open-PR disposition (all 26)

### ① Incorporated — close-with-credit (functionality already on-branch) or straight merge
- **Merge in Phase 3:** `#1732` cli_arg→Result (maintainer-approved) · `#1624` job IDs (green, #1623) · `#1742` scheduler w/o worker (green, #1737) · dependabot `#1772` `#1760` `#1754` · `#1655` Vietnamese README.
- **Close-with-credit (work landed as our own commits):** `#1764` MultiEmail · `#1694` email template inheritance · `#1685` PageResponse.meta · issue-driven: TLS `#1341/#1191`, db::dump `#1691/#1736`, logger `#1753`, gen `#1755/#1729`, i18n `#1758/#1770`, rhai `#1768`.

### ② NOT incorporated into 1.0.0
| PR | Author | What | Why out | Action |
|----|--------|------|---------|--------|
| #1657 | alwayys-afk | trait bounds → `impl AsyncFn` | Reintroduces E0282 in generated auth tests | Defer, credit |
| #1693 | jtwaleson | Priority queue | Superseded (ZSET priority queues already on-branch) | Close-with-credit (verify parity) |
| #1698 | elcoosp | Sea-ORM 2.0 | Superseded by our migration; conflicting/stale | Close, credit |
| #1771 | D-system | Apply auto-formatting | Only 5 fixes adopted; rest is churn | Close, explain partial |
| #1699 | SMCodesP | AWS Lambda deploy | Large unreviewed surface | Defer → milestone |
| #1762 | andriishin | 8 READMEs → NRG template | Big docs restructure; overlaps site rebuild | Defer |
| #1708 | floscodes | popular tasks | Unresolved `groups` table + commented-out assertion | Defer, keep open |
| #1730 | pweaver | Tasks API `&[&String]` | Draft, stale | Defer |
| #1757 | dependabot | rand 0.8→0.9 (loco-new) | Breaking API bump | Defer / verify generator |
| #1752 | AnthonyMichaelTDM | sec: bytes + jsonwebtoken | Draft, security-relevant | Evaluate — land if real advisory |
| #1778/#1779/#1780 | David Szabó | old-site dark-mode/flash/img dims | Site being replaced (decision 2) | Close, thanks + credit |

Deferred feature **issues** (post-1.0 milestone): #1640 multi-tenant · #1674 cache · #1673 service flag · #1720 custom field types · #1766 Rails-style migrations.

---

## Communication debt (contributors waiting on a reply)

All replies are drafted; **post after publish**. Highest-value first: `#1732` (approved, just merge) · `#1624` · `#1742` · `#1770`/`#1758` (D-system bug fixes) · `#1774` (zmilan, fixes own #1773) · `#1764` (untriaged 80d) · `#1694`/`#1693` (jtwaleson) · `#1685` (GoCoder7 — inverse: we owe close-or-ping) · `#1698` (courteous close).

---

## Key file map

| File | Role |
|---|---|
| `RELEASE-1.0.0.md` | Decisions ledger (D1–D11), Part A/B/C, verification log — **source of truth** |
| `REMAINING-WORK.md` | Flat #1–#60 remaining-work inventory, confidence-scored |
| `RELEASE-1.0.0-newwork-replies.md` | Reply drafts for the 4 issues built for 1.0 |
| `RELEASE-1.0.0-verification-replies.md` | Verdicts + replies for triage PRs/issues |
| `CHANGELOG.md` | Assembled `## 1.0.0` (Keep-a-Changelog) |
| `docs-site/content/docs/extras/upgrades.md` | User-facing migration guide (needs 1.0 fill) |
| `xtask/src/versions.rs` | `bump`/publish-order tooling (LOCO_VERSION regex bug) |

---

*Prep by Claude (release-manager mode). Every 🔒 button is Jondot's.*
