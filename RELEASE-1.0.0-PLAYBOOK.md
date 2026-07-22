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

## Phase 2 — Final verification  ·  ✅ DONE (2026-07-23, HEAD `e6db54bd`)

- [x] **Reproduce-or-refute #1749 / #1759 / #1768** — **fixed, runtime-confirmed.** Fresh `loco new` (sqlite/async/serverside) → `shared.ftl` generated at `assets/shared.ftl` (out of `assets/i18n/`, the fix) → `cargo build` clean → boot → `GET /_health` = **200**.  **← go/no-go PASS**
- [x] **Full green gate with `loco-gen --all-features`** — clippy `-D warnings`, `hack check --each-feature`, `test --all-features`, and **`test -p loco-gen --all-features`** (with-db template/model tests incl.) all **EXIT 0**, **no snapshot drift**. loco-new fmt+clippy EXIT 0. *Full wizard matrix (`loco-new` `cargo test`) not re-run on this HEAD — redundant with green 4b + Part A, and it OOM-kills this 16 GB box; run on a higher-RAM host if absolute certainty wanted.*
- [x] **`cargo publish --dry-run`** — `loco-gen` full-verify **EXIT 0**; `loco` package **EXIT 0**; `loco-rs` blocks **only** on `loco-gen ^1.0.0` not yet on crates.io (inherent pre-publish chicken-and-egg; resolves via the staged publish order below). Metadata/packaging clean.
- [x] Verify `loco-new/src/lib.rs` `LOCO_VERSION == "1.0"` — confirmed `"1.0"`. Crate versions `loco-rs`/`loco-gen`/`loco` all `1.0.0`; `sea-orm =2.0.0-rc.41`.

## Phase 3 — GitHub hygiene  ·  `[JONDOT]` presses (drafts ready)

- [ ] Create the **missing `1.0.0` milestone** + `blocker` / `1.0` labels (none exist today).
- [ ] Merge the safe set (decision 4). **Close #1698** (superseded Sea-ORM 2.0); close **#1778/#1779/#1780** (decision 2).
- [ ] Post drafted replies — B1–B6, `RELEASE-1.0.0-newwork-replies.md`, `RELEASE-1.0.0-verification-replies.md` — **after** publish so "fixed in 1.0.0" is true.
- [ ] Move deferred items to the post-1.0 milestone.

## Phase 4 — Publish  ·  `[JONDOT]`

- [ ] Stamp `## 1.0.0 - <publish date>`. Push `release/1.0.0` → release PR → merge to `master`.
- [ ] **Publish in dependency order:** `cargo publish -p loco-gen` → `cargo publish -p loco-rs` → `(cd loco-new && cargo publish)`.
- [ ] `git tag v1.0.0 && git push origin v1.0.0`; `gh release create v1.0.0 --notes-file <1.0.0 CHANGELOG section>`.
- [ ] **Website deploy flip** (decision reversed — new Astro site ships *with* 1.0, not as a fast-follow). Prep is DONE and on-branch (see below); the only button is in the **hosting dashboard** (Cloudflare Pages / Netlify — external to the repo):
  - Build command: `cd website && corepack enable && pnpm install --frozen-lockfile && pnpm build`
  - Publish/output dir: `website/dist`
  - Node: `22`  ·  (old Zola settings — `zola build` / `docs-site/public` — get replaced)
- [ ] Publish the announcement; post release-tied closes/credits.

### Website cutover — big-bang (DONE on-branch, 2026-07-23)
- [x] Merge Astro site into `release/1.0.0` — `0653fc85` (0 conflicts, additive)
- [x] Exclude 60 internal `docs/superpowers/**` planning docs from the release — `3a8608f3`
- [x] Sync 0.16→1.0 upgrade guide into the site via `migrate-docs.mjs` → `rewrite-links.mjs` — `7ffbe7de`
- [x] CI: `docs.yml` now builds + URL-parity-checks the Astro site — `0a8b074b`
- [x] Local verification: `pnpm build` = **82 pages / 0 errors**, `pnpm test` = 25 pass, URL parity **0 missing** (64 docs, 18 blog/casts/authors)
- [ ] **Deploy flip in hosting dashboard** (the one remaining button — above, in Phase 4)
- [ ] *Post-launch follow-up (not release-blocking):* decide whether to retire `docs-site/` as the authoring source and move authoring fully into Starlight (`website/src/content/docs`), rewiring snipdoc/llms-check accordingly.

## Phase 5 — Post-publish fast-follow  ·  same day

- [ ] Clean-machine smoke test: `cargo install loco` → `loco new` → boot (the real user path).
- [ ] Confirm the new site is live and correct at loco.rs after the deploy flip (spot-check docs, blog, upgrade guide, search).
- [ ] Backlog triage wave: label remaining ~20 PRs/issues to the post-1.0 milestone.

---

## Go / No-Go gate (all ✅ before Phase 4)

- [x] Engineering green (Part A)
- [x] Full gate re-run with `loco-gen --all-features` — clippy/hack/test-all-features/loco-gen-all-features **EXIT 0**, no snapshot drift (2026-07-23, `e6db54bd`). *Wizard matrix deferred to higher-RAM host — redundant w/ 4b + Part A.*
- [x] #1749 / #1759 / #1768 — **runtime-confirmed fixed**: fresh `loco new` → build → boot → `/_health` 200; `shared.ftl` at `assets/shared.ftl`.
- [x] `cargo publish --dry-run` — `loco-gen` + `loco` clean; `loco-rs` only pends `loco-gen` being on the index first (staged-publish order handles it).
- [x] Migration guide 1.0-complete — `98e44707`
- [x] CHANGELOG wording honest (no "stable Sea-ORM") — `98e44707`
- [x] Announcement ready — `RELEASE-1.0.0-announcement.md`

**Verdict: GO.** All go/no-go items green. Remaining work is Phase 3/4 button-presses (Jondot) + the pre-publish tree-clean (remove/ignore untracked `website/`, `.superpowers/`, `docs/superpowers/**` before the real `cargo publish`, which refuses a dirty tree).

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
