# Loco Rewrite Program (greenlit 2026-07-04)

Supersedes the Bucket-B/C decisions in `GAP-TO-10.md` under Jondot's weights
(rewrite-first, library-first on a net-LOC bar, value-churn is his call —
see memory `loco-rewrite-library-weights`). All work: local commits only,
Sonnet implements sequentially, Opus governs/verifies/commits.

## Greenlit decisions

| Item | Decision |
|---|---|
| `errors.rs` flat enum | **Rewrite** into HTTP-domain vs infra split (BREAKING — accepted, pre-1.0 window) |
| `remote_ip.rs` | **Swap to axum-client-ip** (−~340 LOC). BREAKING: multi-hop XFF chains no longer walked; single trusted-edge model. Must be documented loudly. |
| bgworker (~5,600 LOC) | **Full rewrite** — one clean SQL driver (collapse pg/sqlt) + factored redis; rebuild its tests alongside |
| Route introspection | Rewrite — record verb+path at registration; delete `describe.rs` Debug-regex |
| Test harness | Rewrite — kill `start_from_boot` `sleep(2s)` → readiness poll; unify fixtures; builder for storage tests |
| `validate.rs` (780) | Rewrite the 6-struct + 2-macro design → cleaner single-generic extractor |
| `format.rs` (825) | Rewrite for further clarity/LOC |
| Shell coverage + footguns | boot/cli/logger tests; mutex-poison→error; restore exhaustive match (drop `unreachable!()`); serial_test; docker-skip; dead moka `sync` feature; stale `XXX` comments |
| Small libs (num-format / axum-valid / tower-http request-id) | **Re-spike first** on the net-LOC bar, then decide |

## Decisions (2026-07-04, after re-spikes)

- **errors.rs**: ONE restructured enum — group client-facing vs infra, exhaustive structural
  status mapping (remove the `_ => 500` catch-all), keep variant names (non-breaking).
- **validate.rs**: **adopt axum-valid** (−41% LOC); keep Loco's two error tiers via thin wrappers.
  BREAKING: drops the custom `ValidatorTrait` (validate-without-`validator`-derive).
- **route introspection**: **additive** verb-aware get/post wrappers; old code keeps the regex fallback.
- **num-format**: ~~ADOPT~~ **REJECTED on implementation review.** It's a NEW dep (not
  transitive), the incumbent grouping is already correct (`-0.123` → `"-0.123"` verified — the
  `-0` bug was introduced *by* num-format, which drops the sign), and it saves only ~14 LOC of
  trivial grouping while forcing sign-handling back in. Net: a dep tree for negligible gain,
  against the lean-dep value. `number.rs` unchanged.
- **tower-http request-id**: REJECT (−1 LOC wash + layer-order footgun). Keep hand-rolled.

## Execution order (dependency + risk)

0. **Re-spikes** (parallel, scratchpad-only) — real LOC for the 3 small libs → fold into scope.
1. **Test harness readiness-poll** — foundational, isolated, lifts many tests. *(first)*
2. **Route introspection** — isolated; needs a design call on the registration API.
3. **remote_ip → axum-client-ip** — isolated; loud breaking docs.
4. **errors.rs rewrite** — touches `controller/mod.rs` IntoResponse + `format.rs`.
5. **format.rs rewrite** — after/with errors.
6. **validate.rs rewrite** — isolated.
7. **bgworker full rewrite** — biggest; test suite rebuilt with the new harness.
8. **Shell coverage + footguns** — fold in throughout; final sweep.

## OUTCOME (2026-07-04) — program complete

**Shipped (11 commits, all local, all verified):**
- Test harness: `sleep(2s)` → readiness poll (middleware suite 50s → 3.3s).
- `remote_ip` → axum-client-ip (−42 LOC, dropped `ipnetwork`, breaking: single trusted-edge).
- `errors.rs`: exhaustive Error→HTTP mapping (no silent-500 wildcard; found 4 nested ModelError
  variants; non-breaking).
- Footgun batch: mutex-poison recovery, exhaustive cli match, dead moka `sync` dropped,
  serial_test, stale `XXX` deleted, clippy `-D warnings` now clean.
- Route introspection: additive verb-explicit `Routes::{get,post,…}` methods.
- bgworker: SAFE subset — hoist `to_job`/`ping` + fix 2 real pg/sqlt inconsistencies (enqueue
  tag-error, complete_job run_at). `dequeue`/`initialize_database` left untouched.
- Coverage: logger.rs (0→7 tests), on_shutdown worker-mode guard (behavior-identical seam).

**Rejected on "prove why not" review (governor calls, reported to Jondot):**
- **num-format** — new dep, incumbent already correct; ~14 LOC for a whole dep tree.
- **validate.rs / axum-valid** — validate.rs already DRY; `validator` (present) already does the
  job; axum-valid adds nothing but a dep + drops the `ValidatorTrait` feature for flat LOC.
- **format.rs** — already single-sourced in the prior remediation; a second rewrite is churn.
- **bgworker FULL unification** — UNSAFE: pg `SKIP LOCKED` vs sqlite advisory-lock `dequeue` are
  different concurrency primitives; forcing them risks job double-processing. Real win was ~40 LOC
  + the 2 bug fixes, not the ~−1,000 the duplication's size implied.

**Final gate:** fmt clean · clippy `-D warnings` clean (lib+tests, all backends) · 460 lib tests pass.

## Gate (every step)
`cargo fmt --check` · `cargo clippy --all-features --all-targets -- -D warnings` ·
targeted `cargo test --all-features`. Atomic commit per step. Update CHANGELOG for each
breaking change with the crystallized behavior delta.
