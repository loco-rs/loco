# 1.0.0 — verification block: verdicts + reply drafts (2026-07-14)

The 4 triage PRs + onboarding-cluster issues, each verified against
`release/1.0.0`. Post the reply and close/merge as noted when 1.0.0 ships.

## Summary

| # | Type | Verdict | Action |
|---|------|---------|--------|
| #1758 / #1749 | PR/issue | fixed-in-1.0.0 (`0e6fe874`) | close both, credit @zjom @lunfel |
| #1770 / #1759 | PR/issue | fixed-in-1.0.0 (same commit, better approach) | close as superseded, credit |
| #1768 | issue | fixed-in-1.0.0 (`a640a97f`, rhai 1.25) | close as fixed |
| #1729 | issue | fixed-in-1.0.0 (`f9b87a68`+`5a44657c`) | close as fixed |
| #1755 | issue | **was a real bug — FIXED this session** (`47d03916`) | close as fixed, credit @labike |
| #1708 | PR | defer post-1.0 (mystery `groups` table + commented-out assertion) | reply, keep open |
| #1771 | PR | partial — **5 clean fixes adopted** (`11ce376e`), rest superseded | close, credit @D-system |

---

## PR #1758 / issue #1749 — i18n shared.ftl boot failure — CLOSE (fixed)

**Verdict:** fixed on branch by `0e6fe874` (moves `shared.ftl` out of the scanned
`assets/i18n/` dir; credits Mathieu + zjom). PR #1758's core fix is the same idea;
its CI hardening (`cargo test --all-features` on the generated app) also landed.

> **PR #1758:** Thanks @zjom — and @lunfel for the regression-test gap and the
> fix! This is already resolved on `release/1.0.0` (`0e6fe874`, "keep shared i18n
> resource outside the scanned locale dir"): `shared.ftl` moved to
> `assets/shared.ftl` so the locale scanner never double-registers it — the same
> approach you took. I also folded in your CI-hardening idea (`cargo test
> --all-features` now runs against the generated app), and credited you both as
> co-authors. Closing in favor of that fix since 1.0.0 supersedes it — really
> appreciate the diagnosis.

> **Issue #1749:** Fixed for 1.0.0 — thanks @Leandros for reporting and @zjom /
> @lunfel for the fix (#1758). Root cause: `shared.ftl` living inside
> `assets/i18n/` was picked up by the locale scan *and* `.shared_resources()`,
> which newer fluent-templates rejects as a duplicate. Fixed by relocating it to
> `assets/shared.ftl` (`0e6fe874`). Closing.

## PR #1770 / issue #1759 — SSR FTL resources fail — CLOSE (superseded)

**Verdict:** same root cause + fix as #1758/#1749; branch went with "move out of
i18n dir" over #1770's "rename to `_shared.ftl`" (more robust vs filename-parsing).

> **PR #1770:** Thanks @D-system for tracking this down and the alternate fix!
> Already resolved on `release/1.0.0` (`0e6fe874`) — we moved `shared.ftl` out of
> `assets/i18n/` entirely rather than renaming to `_shared.ftl`, since keeping it
> out of the scanned dir is a bit more robust against fluent-templates filename
> edge cases. CI now also runs `cargo test --all-features` on generated apps to
> catch this class of boot failure. Closing as superseded — appreciate the report
> and fix. 🙏

> **Issue #1759:** Thanks @JLouisa and @zebrowy46czat for the root-cause digging
> — fluent-templates' `ArcLoader::build()` parses every `.ftl` stem as a BCP-47
> tag, so `shared.ftl` was registered twice. Fixed on `release/1.0.0`
> (`0e6fe874`) by moving the shared resource out of `assets/i18n/`. Closing.

## issue #1768 — `cargo install loco` fails — CLOSE (fixed)

**Verdict:** not a naming issue — a dep-resolution break (`rhai` floor `1.23`
didn't build against newer 1.x). Fixed by `a640a97f` (rhai → 1.25). Crate/bin
name `loco` is correct; install command unchanged.

> Thanks for the report — fixed for 1.0.0. It wasn't a `loco`/`loco-cli` naming
> issue; `cargo install loco` re-resolves deps against crates.io (ignoring our
> lockfile) and was picking a `rhai` release that no longer built against our
> pinned floor. We bumped the floor to `rhai = "1.25"` (`a640a97f`), included in
> 1.0.0. `cargo install loco` will work with no `--locked` workaround once 1.0.0
> publishes. Closing as fixed.

## issue #1729 — migration FK name mismatch — CLOSE (fixed)

**Verdict:** fixed by `f9b87a68` + `5a44657c` — `create_table` now names inline
FKs `fk-{child}-{ref}-to-{parent}`, matching `add_reference`/`remove_reference`;
regression test added.

> Thanks for the detailed repro — real bug. `create_table`'s inline FK naming was
> parent-first while `add_reference`/`remove_reference` name FKs child-first, so a
> FK made by `create_table` could never be dropped by `remove_reference`. Fixed
> for 1.0.0: `create_table` now names inline FKs `fk-{child}-{ref}-to-{parent}`
> (matching the other two), plus a related table-name normalization fix and a
> regression test that creates + removes an inline reference against a real DB.
> With your `task_templates`/`departments` repro the names now match. Closing as
> fixed.

## issue #1755 — controller/model gen: no primary key — CLOSE (fixed this session)

**Verdict:** was a **real, still-open bug** — a field-less model/scaffold omitted
its `id` PK → non-compiling entity. Fixed this session (`47d03916`): `id` is now
unconditional in the migration template, with a regression test.

> Reproduced — thanks @labike. The trigger is generating a **model or scaffold
> with no fields** (`cargo loco generate model posts`): our migration template
> only added the `id` primary key when at least one other field was present, so a
> field-less model produced a table with no PK and an entity that fails to compile
> — exactly the errors you saw. (Your `generate controller` command only writes
> the controller; the pasted entity came from a field-less model/scaffold — the
> naming was a red herring, but the bug was real.) Fixed for 1.0.0: `id` is now
> always emitted, with a regression test for the zero-field case. Closing as
> fixed.

## PR #1708 — popular tasks — DEFER (keep open)

**Verdict:** `user:delete` is a fine idea, but unmergeable as-is — it injects an
unexplained `groups` migration into every generated app to paper over a test
failure, and comments out a load-bearing assertion in `wizard/new.rs`. Root cause
was never found; "changes requested" sat 4+ months.

> Hi @floscodes — thanks for sticking with this. `user:delete` alongside
> `user:create` is a genuinely nice addition and I'd like to land it, but not as-is
> for 1.0: adding a whole `groups` table to every generated app's schema to work
> around a test failure — without knowing why the test needed it — isn't something
> we can commit to at a 1.0 stability boundary. I think the real fix is on the
> wizard-test side (the `CreateJoinTableUsersAndGroups` step shouldn't create an
> implicit runtime dependency for an unrelated task test), not in the generated
> app. Could you check whether `ActiveModelTrait::delete` on `users::Model`
> actually touches a `groups` relation, or if the test is just bleeding state from
> a prior wizard step? Also, `run_test()`'s assertion got commented out in favor
> of a `println!` — that masks the failure, so let's not merge that. I'm going to
> defer this past 1.0 rather than adopt it now, but I'd love to bring `user:delete`
> in right after, once the `groups` mystery is actually resolved. Appreciate the
> persistence!

## PR #1771 — auto-formatting — CLOSE (partially adopted)

**Verdict:** mixed. `schema.rs` hunk already superseded by the rebuild; `cli.rs`
hunks partly-applied / now need two occurrences. The 5 clean pieces (loco-new
`is_some_and`, xtask `.clone()`/`const`/`#[must_use]`/stray-`;`) were adopted this
session (`11ce376e`, credited).

> Thanks @D-system — appreciate isolating the formatting/clippy cleanup out of
> feature PRs. Against `release/1.0.0` it's a mixed bag after the 1.0 generator
> rebuild: the `schema.rs` change is already superseded (the rebuild restructured
> that enum match the same way and further), and a couple of the `cli.rs` one-liners
> are already on the branch while the `port.map_or → unwrap_or` fix now needs to
> land in two places. I've pulled the still-applicable pieces in with credit — the
> loco-new `is_some_and`, and the xtask `.clone()`/`const fn`/`#[must_use]`/stray-`;`
> fixes. Closing since the remainder is overtaken by the rebuild — thanks again for
> keeping formatting out of feature PRs, it makes reviews much easier.
