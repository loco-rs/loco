# Loco 1.0.0 — Complete Remaining-Work Inventory

Companion to `RELEASE-1.0.0.md`. **Every** open item, flat, individually
actionable, each scored by *my* ability to execute it correctly, confidently,
and properly — myself. Ground-truthed against live GitHub state on 2026-07-14.

## Legend

**Type** — what the deliverable is:
- ✍️ **Draft** — reply / notes / doc text I write in full.
- 🔎 **Verify+Draft** — I investigate the branch to confirm/deny a claim, *then* draft.
- 🔧 **Code** — an actual code/doc change on the branch.
- 🔒 **Prep-only** — governance-blocked *button* (push / merge / post comment /
  publish / tag / org-transfer). I prepare it to ready; **you** press. The score
  is for the prep, not the press.

**Confidence (1–10)** — how sure I am the deliverable will be *correct* once I
finish it. 9–10 = mechanical/known. 7–8 = clear branch evidence, low residual
risk. 5–6 = root cause may be subtle; I can reach a verdict but it might be
"needs your judgment." <5 = I'd be guessing.

Engineering (Part A) is 100% done and is **not** relisted here. This is Part B
(correspondence) + Part C (publish) + newly-discovered items.

---

## 1. Adopted PRs → close-with-credit (code already on the branch)

Each = post the drafted reply + close. Code is shipped, so confidence is about
the reply being accurate. All still OPEN on GitHub.

| # | PR | Author | Deliverable | Type | Conf |
|---|----|--------|-------------|------|-----|
| 1 | #1698 Sea-ORM 2.0 | elcoosp | Close: "seeded 1.0.0, credited" | ✍️+🔒 | 10 |
| 2 | #1693 priority queue | jtwaleson | Close: adopted (ZSET, priority col) | ✍️+🔒 | 10 |
| 3 | #1685 PagerMeta | GoCoder7 | Close + closes #1683 | ✍️+🔒 | 10 |
| 4 | #1742 scheduler w/o worker | mccormickt | Close + closes #1737 | ✍️+🔒 | 10 |
| 5 | #1774 mailer implicit TLS | zmilan | Close + closes #1773 | ✍️+🔒 | 10 |
| 6 | #1624 job IDs from perform_later | NewtTheWolf | Close + closes #1623 | ✍️+🔒 | 10 |
| 7 | #1694 email inheritance | jtwaleson | Close + closes #1652 (credited, A5) | ✍️+🔒 | 10 |
| 8 | #1764 MultiEmail | YtimoDeng | Close (credited, A5) | ✍️+🔒 | 10 |
| 9 | #1732 cli_arg Result | dsgallups | Close (credited, A5, breaking) | ✍️+🔒 | 10 |
| 10 | #1772 checkout v6→v7 | dependabot | Close as done (CI) | 🔎+🔒 | 9 |
| 11 | #1760 sccache-action | dependabot | Close as done (CI) | 🔎+🔒 | 9 |
| 12 | #1757 rand 0.8→0.9 | dependabot | Verify branch rand=0.9 then close | 🔎+🔒 | 9 |

## 2. Superseded / already-fixed on the branch → verify + close

| # | PR/Issue | Deliverable | Type | Conf |
|---|----------|-------------|------|-----|
| 13 | #1752 sec: bytes+jsonwebtoken | **Verified**: branch is jsonwebtoken 10.4.0 / bytes 1.12.0 → close as "addressed in 1.0.0". No code. | 🔎+🔒 | 9 |

## 3. Deferred PRs → drafted "not now" replies (keep open / own release)

| # | PR | Author | Deliverable | Type | Conf |
|---|----|--------|-------------|------|-----|
| 14 | #1657 AsyncFn helpers | alwayys-afk | "deferred, not rejected" (D7 confirmed) + credit | ✍️+🔒 | 9 |
| 15 | #1730 Tasks API [&String] | pweaver | "still a draft; post-1.0" (note: overlaps #1732) | ✍️+🔒 | 9 |
| 16 | #1699 AWS Lambda | SMCodesP | "big surface; own release" keep open | ✍️+🔒 | 9 |
| 17 | #1762 NRG README tooling | andriishin | "post-1.0 docs-infra pass" (D10) | ✍️+🔒 | 9 |

## 4. PRs that still need a real triage decision (I investigate → recommend → draft)

These are the only PRs where I owe you a *judgment*, not just a reply.

| # | PR | Author | What I need to determine | Type | Conf |
|---|----|--------|--------------------------|------|-----|
| 18 | #1771 auto-formatting | D-system | Is it just `fmt` on files the rewrite already reformatted? Likely close-as-superseded. | 🔎+🔒 | 8 |
| 19 | #1770 i18n boot fix | D-system | Is the i18n/FTL boot error already fixed on the branch? Close-as-fixed or adopt. | 🔎+🔒 | 7 |
| 20 | #1758 new-project generator fix | zjom | Covered by the generator rebuild? (fixes #1749) Verify → credit/close or adopt. | 🔎+🔒 | 7 |
| 21 | #1708 popular tasks | floscodes | Read the diff; decide adopt vs defer for 1.0.0. Genuine product call. | 🔎+🔒 | 6 |

## 5. Merge-on-GitHub / social / org (you press; I prep + verify)

| # | Item | Deliverable | Type | Conf |
|---|------|-------------|------|-----|
| 22 | #1754 npm `yaml` bump | Verify clean dependabot bump; **you** click Merge (D11) | 🔒 | 9 |
| 23 | #1655 Vietnamese README | Verify it renders + links; **you** merge (D9, sets policy) | 🔒 | 8 |
| 24 | #1714 donate `rhai-loco` | Draft accept reply; **you** do the org transfer (D8) | 🔒 | 8 |

## 6. Issues → close as fixed-by-adopted-PR (mechanical)

| # | Issue | Closed by | Type | Conf |
|---|-------|-----------|------|-----|
| 25 | #1773 SMTP implicit TLS | #1774 | ✍️+🔒 | 9 |
| 26 | #1737 scheduler w/o worker | #1742 | ✍️+🔒 | 9 |
| 27 | #1623 job ID from perform_later | #1624 | ✍️+🔒 | 9 |
| 28 | #1683 PagerMeta | #1685 | ✍️+🔒 | 9 |
| 29 | #1652 email templating | #1694 | ✍️+🔒 | 9 |

## 7. Issues → close as fixed-by-1.0.0 (each needs a branch verification first)

| # | Issue | What to verify on the branch | Type | Conf |
|---|-------|------------------------------|------|-----|
| 30 | #1749 generated project won't start | A3 boot already proves fresh app starts → confirm the exact symptom | 🔎+🔒 | 8 |
| 31 | #1751 better error handling | `#[non_exhaustive] Error` + error narrowing shipped → matches the ask | 🔎+🔒 | 8 |
| 32 | #1768 `cargo install loco` fails | Root-cause the install failure vs the loco-cli removal / crate naming | 🔎+🔒 | 6 |
| 33 | #1759 SSR FTL resources fail | Find the i18n/fluent boot path; confirm fixed (ties to #1770) | 🔎+🔒 | 6 |
| 34 | #1755 controller gen: no primary key | Reproduce against the rebuilt generator; fixed or still open? | 🔎+🔒 | 6 |
| 35 | #1729 migration FK name mismatch | Check create_table vs remove_reference FK naming in loco-gen | 🔎+🔒 | 6 |
| 36 | #1736 seed dump fails on SQLite | Reproduce seed-dump on SQLite; may still be open | 🔎+🔒 | 5 |
| 37 | #1727 development.yml YAML evals/prettier | serde_yaml_ng parse of eval/long-line YAML; may still be open | 🔎+🔒 | 5 |

## 8. Issues → RE-SCORED on value + build-confidence (2026-07-14)

The 10 "post-1.0 triage" issues were re-evaluated purely on **feature value**
and **my confidence in a correct implementation** (scope/effort ignored, per
owner). Four crossed the bar and are **BUILT for 1.0.0** (committed locally,
green-gated); replies drafted in `RELEASE-1.0.0-newwork-replies.md`. The rest
stay out for evidenced reasons (superseded / contested-pattern / genuine design
uncertainty), not hand-waving.

**Built for 1.0.0 — close as fixed/implemented (reply drafts ready):**

| # | Issue | Value | Conf | Commit |
|---|-------|:---:|:---:|--------|
| 46 | #1341 Redis over TLS | 8 | 9 | `3f773108` |
| 47 | #1191 Postgres TLS/SSL | 7 | 9 | `3f773108` |
| 41 | #1691 reliable seed dumping (+ #1736 bug) | 7 | 8 | `551e82fa` |
| 45 | #1753 make internals public (narrow: logger) | — | 9 | `fb545ed9` |

**Stay post-1.0 — ack reply + milestone (you apply label):**

| # | Issue | Value | Conf | Why not now |
|---|-------|:---:|:---:|-------------|
| 40 | #1720 custom field types | 5 | 5 | SeaORM 2.0 entity-first may moot the mechanism; docs workaround exists |
| 42 | #1674 multi-layer cache | 5 | 7* | conf drops if cross-instance coherence required; thin demand |
| 44 | #1640 multi-tenant / multi-app | 5 | 5 | conflated ask; needs an AppContext design spike (RLS vs pool-per-tenant) |
| 38 | #1766 "Rails-style" migrations | 4 | 8 | actually a `ColType` enum-bloat cleanup; low value, unvalidated demand |

**Decline (evidenced):**

| # | Issue | Value | Conf | Why |
|---|-------|:---:|:---:|-----|
| 43 | #1673 `--service` flag | 3 | 5 | contested pattern, contradicts loco's fat-model convention; offer docs recipe |
| 39 | #1761 api+template scaffold | 2 | 4 | superseded by adaptive SPA scaffold (commit `d3d9eeed` deleted html/htmx kinds) |

## 9. Issues → close as invalid

| # | Issue | Type | Conf |
|---|-------|------|-----|
| 48 | #1739 "Hello," (empty) | ✍️+🔒 | 10 |

## 10. Docs / release-artifact work I can do now (real deliverables)

| # | Item | Deliverable | Type | Conf |
|---|------|-------------|------|-----|
| 49 | Migration guide: A5 breaking changes | Add `cli_arg` + `Template::new` fallible + mailer-scaffold notes to the 0.16→1.0 guide | 🔧 | 8 |
| 50 | CHANGELOG date stamp | `## 1.0.0` → `## 1.0.0 - <publish date>` at publish | ✍️ | 10 |
| 51 | GH release notes file | Produce the `--notes-file` from the CHANGELOG 1.0.0 section | ✍️ | 9 |
| 52 | Announcement draft | Blog/Discord/X copy — needs your voice + channels | ✍️ | 6 |
| 53 | `cargo publish --dry-run` (×3) | Verify loco-gen/loco-rs/loco are publishable (metadata, path+version deps) before you publish | 🔎 | 8 |

## 11. Publish runbook — governance-blocked buttons (you execute; I've prepped)

| # | Step | Type | Conf (prep) |
|---|------|------|-----|
| 54 | `git push origin release/1.0.0` | 🔒 | 10 (ready) |
| 55 | Open release PR → merge to master | 🔒 | 10 |
| 56 | `cargo publish -p loco-gen` → `loco-rs` → `loco` | 🔒 | 8 (dry-run first, #53) |
| 57 | `git tag v1.0.0 && push` | 🔒 | 10 |
| 58 | `gh release create v1.0.0 --notes-file …` | 🔒 | 9 (notes from #51) |
| 59 | Post all §1–§9 replies; close PRs/issues | 🔒 | 9 (drafts ready) |
| 60 | Publish announcement | 🔒 | 6 (draft #52 first) |

---

## What this means

- **Everything I can fully own is drafting + verification** — items #1–#53. Of
  those, the only ones carrying real *judgment* risk are the 4 triage PRs
  (#18–#21) and the 6 subtle issue verifications (#32–#37); the rest is
  mechanical or already-evidenced.
- **Every 🔒 is a button I'm barred from pressing** (push/merge/post/publish/
  tag/org-transfer). I can get all of them to "ready"; you press.
- **Highest-value next block I can execute right now, unattended:** the 4 triage
  PRs (#18–#21) + the 8 issue verifications (#30–#37) — turning every 6–8 into a
  concrete verdict + finished reply, so nothing in Part B is left to judgment at
  publish time.
