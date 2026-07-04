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

## Gate (every step)
`cargo fmt --check` · `cargo clippy --all-features --all-targets -- -D warnings` ·
targeted `cargo test --all-features`. Atomic commit per step. Update CHANGELOG for each
breaking change with the crystallized behavior delta.
