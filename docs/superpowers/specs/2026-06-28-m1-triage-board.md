# M1 Triage Board — PRs & Issues

Date: 2026-06-28. 23 open PRs, 25 open issues. Recommendation column is for the
maintainer to act on (this session makes local commits only — no pushes/merges).

Legend: ✅ land in M1 · 🔬 review for M2 · 🧭 needs your decision · 🗑 close

## Issue ↔ PR map (dedup)

- **#1749 + #1759** (fresh project won't boot — "Failed to add FTL resources")
  → fixed locally; community PRs **#1758** (zjom — same fix + CI hardening) and
  **#1770** (keeps file in `i18n/`, weaker). **One fix closes both issues.**
- **#1768** (`cargo install loco`) → fixed locally (rhai 1.25). No existing PR.
- **#1773** (SMTP implicit TLS / 465) → PR **#1774**.
- **#1737** (scheduler without worker) → PR **#1742**.
- **#1623** (job IDs from `perform_later`) → PR **#1624**.
- **#1652** (email templating) → PR **#1694** (Tera inheritance in emails).
- **#1683** (`PagerMeta` on `PageResponse`) → PR **#1685**.

## PRs

### ✅ Land in M1 (safe / stabilizing)
| PR | What | Note |
|----|------|------|
| #1758 | i18n fix (#1749/#1759) + sanity-CI hardening | **Adopt** (credit zjom). Trim scope creep: drop unrelated `schema.rs` match-guard + `cli.rs`/`template.rs` cosmetics. |
| #1752 | sec: jsonwebtoken 9→10 (CVE auth-bypass), bytes, testcontainers | **Draft** — needs un-draft. ⚠️ jsonwebtoken 10 makes crypto backend a *feature*; verify `auth_jwt` enables a default backend (non-breaking). |
| #1772 | bump actions/checkout 6→7 | dependabot, trivial |
| #1760 | bump sccache-action | dependabot, trivial |
| #1757 | bump rand 0.8→0.9 in loco-new | dependabot; check code compiles |
| #1754 | bump yaml in docs-site | dependabot, docs only |
| #1771 | apply auto-formatting | trivial if it matches rustfmt config |

### 🔬 Review for M2 (feature work that fits roadmap)
| PR | What | Maps to |
|----|------|---------|
| #1774 | mailer implicit TLS (SMTPS 465) | #1773 |
| #1764 | MultiEmail / multi-recipient templates | — |
| #1742 | scheduler+server without worker | #1737 |
| #1693 | priority queue (+1115/-159, BEHIND) | — |
| #1624 | return job IDs from `perform_later` | #1623 |
| #1657 | replace trait fn bounds with `impl AsyncFn` | modernization |
| #1694 | Tera inheritance in emails (BEHIND) | #1652 |
| #1699 | AWS Lambda deploy support (+600) | — |
| #1685 | `PagerMeta` on `PageResponse` | #1683 |
| #1708 | popular tasks | — |
| #1732 | `Vars::cli_arg` → `Result<&str>` | small refactor |
| #1730 | Tasks API takes `[&String]` (draft, BEHIND) | — |

### 🧭 Needs your decision (big / strategic)
| PR | What | Why |
|----|------|-----|
| #1698 | **Sea-ORM 2.0** | CONFLICTING/DIRTY, major upgrade. Likely breaking → conflicts with "no API breaks". Defer or schedule as its own track. |
| #1762 | migrate 8 READMEs to single NRG template (+922) | Overlaps M4 docs rewrite. Decide before merging to avoid rework. |
| #1655 | Vietnamese README translation | Docs; merge or fold into M4. |

### 🗑 Likely close
- Issue **#1739** ("Hello,") — empty/junk.

## Issues — themes for M2/M3/M4
- **TLS gaps** (recurring): Postgres TLS #1191, Redis TLS #1341, SMTP TLS #1773.
  Worth a unified "TLS everywhere" mini-epic in M2.
- **Generator/migrations**: #1766 (Rails-style migrations), #1761 (api+template
  scaffold), #1755 (controller w/o PK), #1729 (FK naming), #1720 (custom field
  types), #1673 (`--service` flag). Cluster → generator improvements in M2.
- **Seeding**: #1736 (SQLite fixture seeding fails), #1691 (reliable seed dump).
- **DX/architecture**: #1751 (better error handling), #1753 (make internals
  public), #1674 (multi-layer cache), #1640 (multi-tenant), #1683 (PagerMeta).
- **Ecosystem**: #1714 (adopt rhai-loco into org).
