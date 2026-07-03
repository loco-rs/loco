# Area: A4 · Views & Templating

## Scope (files reviewed, with LOC)

- `src/controller/views/mod.rs` (97) — `ViewRenderer` trait, `ViewEngine<E>` wrapper, `template()` one-off render, `FromRequestParts` extractor, feature-flag engine re-export.
- `src/controller/views/engine.rs` (314; ~204 impl + ~110 tests) — filesystem `TeraView` with debug-mode hot reload via `notify`.
- `src/controller/views/engine_embedded.rs` (135; 0 tests) — compile-time-embedded `TeraView` (build.rs-generated templates).
- `src/controller/views/tera_builtins/mod.rs` (1), `tera_builtins/filters/mod.rs` (7) — filter registration.
- `src/controller/views/tera_builtins/filters/number.rs` (182; ~118 impl + ~64 tests) — `number_with_delimiter`, `number_to_human_size`, `number_to_percentage` Tera filters.
- `src/controller/views/pagination.rs` (33; 0 tests) — `Pager<T>`/`PagerMeta` DTOs.
- `src/tera.rs` (8; 0 tests) — standalone `render_string()` one-off renderer (used by mailer/config, not by `views/`).

Total ≈777 lines including tests, matching the AREAS.md estimate.

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 5 | Real feature (hot reload) is clean, but a dead, self-admitted-buggy locale block sits commented-out in the extractor (`mod.rs:83-92`), and the two engines disagree on their post-process API shape (`engine.rs:49` static ctor vs `engine_embedded.rs:37` instance builder). |
| 2. Economy of concepts | 6 | The fs/embedded engine split is structurally justified (only one compiles per feature flag, not real runtime duplication) — but there are **two** independent "one-off template render" helpers doing the same job: `controller::views::template()` (`mod.rs:55-61`) and `crate::tera::render_string()` (`tera.rs:5-8`). |
| 3. Low LOC | 6 | Reasonably lean; hot-reload complexity (`engine.rs:102-167`) is warranted by its feature. Avoidable duplication: `DEFAULT_ASSET_FOLDER` literal defined twice (`engine.rs:13`, `engine_embedded.rs:6`) and the two one-off renderers above. |
| 4. Non-brittle | 4 | `ViewEngine::from_request_parts` has `Rejection = Infallible` (`mod.rs:74`) yet panics via `.expect("TeraLayer missing...")` (`mod.rs:82`) if the layer isn't installed — the type signature promises no failure while the implementation can crash the request thread. Number filters silently no-op on non-numeric input (`number.rs:61`, `number.rs:115`) rather than erroring. |
| 5. Maintainable | 5 | Domain boundary (`ViewRenderer`/`ViewEngine`/engine impl) is clear and cohesive, but the builder-API divergence between engines (`engine.rs:49` `build_with_post_process` vs `engine_embedded.rs:37` `post_process`) means user code written against one engine does not compile against the other — a real maintenance trap when toggling `embedded_assets`. |
| 6. Correctness | 5 | `number.rs` has strong `rstest`-parameterized coverage (`number.rs:128-181`, 20+ cases incl. negatives/decimals). But `engine_embedded.rs` has **zero** tests, `pagination.rs` has zero tests, `tera.rs` has zero tests, and `engine.rs`'s `#[cfg(not(debug_assertions))]` release-mode render branch (`engine.rs:200-201`) is never exercised — CI has no release-mode test job (checked `.github/workflows/*.yml`, no match), so cargo test always compiles the debug/hot-reload branch only. |
| 7. No reinvented wheels | 6 | `number_to_human_size` correctly delegates to the `byte-unit` crate (`number.rs:79-89`, incl. an explicit v4→v5 migration shim comment at `number.rs:82-83`) — good practice. `number_with_delimiter`'s thousands-separator (`number.rs:9-39`) is hand-rolled; a crate could replace it but see risk note below. |
| **Overall** | **6** | Solid, purposeful core abstraction (trait + wrapper + swappable engine) let down by a leftover dead/buggy code block, a forced-panic extractor contract, an API mismatch between the two engine variants, and thin test coverage on the embedded engine and the release code path. |

## Evidence log

1. **FACT**: `mod.rs:69-96` defines `impl FromRequestParts for ViewEngine<E>` with `type Rejection = std::convert::Infallible` (`:74`), but the body calls `.expect("TeraLayer missing. Is the TeraLayer installed?")` (`:82`) on a fallible `Extension::from_request_parts`.
   **Judgment**: The type system advertises "this extraction cannot fail" while the implementation panics (500/thread abort) if the middleware wiring is wrong — a footgun for any app that forgets to install `TeraLayer`. Should either be a real `Rejection` type or documented loudly.
   **KPI(s)**: 4 (non-brittle), 6 (correctness). **Severity: HIGH** (crashes user requests, not just this area's tests).

2. **FACT**: `mod.rs:83-92` contains a commented-out block that manipulates `Accept-Language`/locale context and is annotated inline `// BUG: this does not mutate or set anything because of clone`.
   **Judgment**: Dead code with an admitted, unresolved bug left in the live tree — classic patch-on-patch residue signaling an abandoned half-feature (locale-aware default context) rather than a clean removal or fix.
   **KPI(s)**: 1 (holistic vision), 5 (maintainable). **Severity: MEDIUM** (dead code, not executed, but pollutes the extractor's readability and hints at missing i18n support).

3. **FACT**: `engine.rs:49-56` exposes `TeraView::build_with_post_process(post_process)` as a static constructor; `engine_embedded.rs:37-43` exposes `TeraView::post_process(self, post_process)` as a consuming instance method with a different name, signature (`FnMut` vs `Fn`), and calling convention.
   **Judgment**: These are the two variants of literally the same feature (post-process the Tera instance after building), yet they are not interchangeable — code written for one engine will not compile if the `embedded_assets` feature is toggled. This directly contradicts the premise that the two engines are a transparent feature-flag swap. (Independently corroborated by a prior inventory pass: `docs/superpowers/specs/2026-07-03-loco-1.0-inventory/00-MASTER-coverage-matrix.md:22` flags "`build_with_post_process` missing" on the embedded side.)
   **KPI(s)**: 1 (holistic vision), 5 (maintainable). **Severity: HIGH** (breaks the "just flip a feature flag" promise of the architecture).

4. **FACT**: `mod.rs:55-61` (`views::template()`, uses `tera::Tera::default()` + `render_str`) and `tera.rs:5-8` (`tera::render_string()`, uses `Tera::one_off`) both implement "render a one-off Tera template string with a serializable context," in two different modules, with two different signatures (`S: Serialize` generic vs `&serde_json::Value`).
   **Judgment**: Unnecessary concept duplication — a maintainer fixing/extending one-off rendering (e.g., adding a shared filter set) must remember to touch both. `tera.rs::render_string` is used by `mailer/template.rs:69-71` and `config/mod.rs:170`; `views::template` is used by `controller/format.rs:209,331`. Neither registers the custom `tera_builtins` filters, so both are consistent with each other in that respect, but the split itself is unjustified.
   **KPI(s)**: 2 (economy of concepts), 3 (low LOC). **Severity: MEDIUM**.

5. **FACT**: `engine_embedded.rs` (135 lines, the entire embedded-engine implementation including its own bespoke `render()` error-handling branch at `:118-132` that inspects the error string for `"not found"`) has no `#[cfg(test)]` module — grep for `#[test]|#[rstest]` returns 0 hits, vs. `engine.rs` which has 2 tests covering both plain render and hot-reload (`engine.rs:213-234`, `:237-312`).
   **Judgment**: The parallel implementation that ships to every embedded-assets consumer is untested at the unit level (only the build.rs codegen is tested, via `tests/build_scripts/embedded_assets.rs`, not `TeraView::render`/`from_embedded_templates` themselves).
   **KPI(s)**: 6 (correctness). **Severity: MEDIUM-HIGH**.

6. **FACT**: `number.rs:54-63` (`number_with_delimiter`) and `number.rs:105-117` (`number_to_percentage`) both fall through to `Ok(value.clone())` / pass-through on non-numeric input, without any error or warning, per the doc comments at `:50-53` and `:101-104` ("will return the original value as a string without any error").
   **Judgment**: This is a documented, deliberate design choice (Rails-view-helper parity — never break template rendering on a bad filter argument), which mitigates severity, but it is still "silent-wrong" behavior per the rubric's KPI4 definition: a template author who passes the wrong type gets no signal that the filter did nothing.
   **KPI(s)**: 4 (non-brittle). **Severity: LOW** (intentional and documented, but still a silent no-op).

## Patch-on-patch smells (specific, cited)

- Dead, self-documented-buggy code block: `mod.rs:83-92` (`// BUG: this does not mutate or set anything because of clone`).
- Version-drift shim comment: `number.rs:82-83` — explicit note that `byte-unit` v5's default precision behavior changed from v4 and `{:.2}` was added to compensate; confirms `byte-unit = "5.1"` in `Cargo.toml:120` was a live migration, not a from-scratch design.
- Clippy suppression: `#![allow(clippy::implicit_hasher)]` at `number.rs:1`.
- Duplicated literal: `DEFAULT_ASSET_FOLDER` defined independently in both `engine.rs:13` and `engine_embedded.rs:6` (same value, two sources of truth).
- API divergence between feature-flagged variants: `engine.rs:49` vs `engine_embedded.rs:37` (see Evidence #3).

## Library hypotheses

1. **Hand-rolled**: `separate_with_commas`/`separate_integer_part` thousands-separator logic, `number.rs:9-39`.
   **Candidate crate**: `num-format` (locale-aware thousands separators for numeric types).
   **Why it might be simpler**: Removes ~30 hand-rolled lines of char-indexing/negative-sign logic.
   **Risk / why it might not fit**: `num-format` operates on parsed numeric types (i64/f64), but the current implementation deliberately works on the **original string representation** of the JSON number to preserve arbitrary decimal precision (tested at `number.rs:145`: `1_234_567_890.123_456` → `1,234,567,890.123456`, `:146`: `0.000_123` preserved exactly). A crate that round-trips through `f64` would risk precision loss on these exact test cases. **NEEDS SPIKE** — verify `num-format` (or alternative) can preserve string-exact decimal precision before swapping.

2. **Hand-rolled**: `views::template()` (`mod.rs:55-61`) and `tera::render_string()` (`tera.rs:5-8`) as two separate one-off-render wrappers.
   **Candidate**: not an external-crate hypothesis but an internal-dedup one — collapse to a single function reused by both call sites.
   **Why it might be simpler**: One function, one doc, one behavior to keep consistent (e.g., filter registration, autoescape mode).
   **Risk**: Call sites have slightly different signatures (`S: Serialize` vs `&serde_json::Value`); a shared impl would need a small adapter. **NEEDS SPIKE** — check that mailer/config callers (`mailer/template.rs:69-71`, `config/mod.rs:170`) don't rely on `Tera::one_off`'s specific autoescape-off semantics in a way `render_str` doesn't already match (empirically it does, since neither registers an HTML extension).

## What is genuinely excellent (cited — be specific)

- `number_to_human_size` (`number.rs:78-90`) correctly reaches for the `byte-unit` crate instead of hand-rolling byte-size formatting, and the code openly documents the v4→v5 upgrade compensation (`:82-83`) rather than silently patching around it — exactly the kind of "library reuse done right" the rubric rewards.
- `number.rs` test suite (`:119-182`) is genuinely thorough: 17 `rstest` cases for `number_with_delimiter` alone, covering negatives, multi-group thousands, high-precision decimals, zero, and pass-through-on-invalid — this is real behavior-exercising coverage, not happy-path smoke testing.
- The hot-reload design in `engine.rs:102-167` is a clean, self-contained feature: a `notify` watcher flips a `dirty` bool on content-changing events only (explicitly filtering out `Access`/metadata-only events, `:133-136`), and the render path lazily rebuilds only when dirty (`:184-195`) rather than on every request — a sensible, non-wasteful design.
- `ViewRenderer` (`mod.rs:20-27`) + `ViewEngine<E>` (`mod.rs:29-37`) is a minimal, well-named two-type abstraction that cleanly supports both engine variants and is `Extension`-friendly for Axum — good economy of concepts at the trait boundary itself.

## Top 3 things that would most raise the area's quality

1. Fix the `ViewEngine` extractor contract: either give it a real `Rejection` (e.g., an error response) instead of `Infallible` + `.expect()` (`mod.rs:74,82`), or explicitly document/enforce that `TeraLayer` is mandatory at boot time so the panic can never reach a live request.
2. Unify the two engines' post-processing API (`engine.rs:49` vs `engine_embedded.rs:37`) into one shape so switching `embedded_assets` on/off is a true drop-in swap, and delete or resolve the dead, admitted-buggy locale block (`mod.rs:83-92`).
3. Add unit tests for `engine_embedded.rs` (build/render/error-path), `pagination.rs`, and `tera.rs`, and add a CI job (or `cfg(test)`-independent check) that exercises `engine.rs`'s release-mode (`not(debug_assertions)`) render path, which is currently never compiled by any test run.
