# Area: A3 · Middleware

## Scope (files reviewed, with LOC)

All files read in full, `src/controller/middleware/`:

| File | LOC | Role |
|---|---|---|
| `mod.rs` | 212 | `MiddlewareLayer` trait, `default_middleware_stack`, `Config` struct |
| `cors.rs` | 316 | CORS — thin config wrapper over `tower_http::cors` |
| `secure_headers.rs` | 345 | hand-rolled tower `Service`, preset-based security headers |
| `remote_ip.rs` | 349 | hand-rolled tower `Service`, X-Forwarded-For IP resolution |
| `static_assets_embedded.rs` | 214 | hand-rolled binary-embedded asset server |
| `static_assets.rs` | 147 | thin wrapper over `tower_http::services::{ServeDir,ServeFile}` |
| `request_id.rs` | 137 | hand-rolled `axum::middleware::from_fn`, UUID/header sanitizing |
| `catch_panic.rs` | 107 | thin wrapper over `tower_http::catch_panic::CatchPanicLayer` |
| `etag.rs` | 107 | hand-rolled tower `Service`, header-comparison-only ETag |
| `_archive/content_etag.rs` | 103 | **dead code** — body-hashing ETag, not compiled |
| `fallback.rs` | 102 | thin wrapper over `axum` fallback + `tower_http::services::ServeFile` |
| `logger.rs` | 101 | thin wrapper over `tower_http::trace::TraceLayer` + `add_extension` |
| `limit_payload.rs` | 88 | thin wrapper over `axum::extract::DefaultBodyLimit` |
| `powered_by.rs` | 88 | thin wrapper over `tower_http::set_header::SetResponseHeaderLayer` |
| `format.rs` | 71 | small `FromRequestParts` extractor for Accept/Content-Type sniffing |
| `timeout.rs` | 68 | thin wrapper over `tower_http::timeout::TimeoutLayer` |
| `compression.rs` | 39 | thin wrapper over `tower_http::compression::CompressionLayer` |

Total ~2,594 LOC across 16 live files + 1 archived file (matches AREAS.md's ~2,000 estimate,
archive included brings it to 2,594).

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 6 | Consistent `MiddlewareLayer` trait shape throughout (`mod.rs:44-70`), but two independently-evolved static-asset implementations (`static_assets.rs` vs `static_assets_embedded.rs`) and a superseded-but-undeleted etag implementation (`_archive/content_etag.rs`) show visible patch-on-patch evolution. |
| 2. Economy of concepts | 6 | One trait (`MiddlewareLayer`, `mod.rs:44`) reused cleanly for 12 middlewares — good. But `StaticAssets`/`FolderConfig` is modeled twice in full (`static_assets.rs:24-76` vs `static_assets_embedded.rs:28-79`) instead of once with a shared config type. |
| 3. Low LOC | 5 | Most files are lean (compression.rs 39 LOC, timeout.rs 68 LOC). But `static_assets_embedded.rs:24-79` duplicates ~45 lines verbatim from `static_assets.rs:24-76`, and the dead `_archive/content_etag.rs` (103 LOC) is pure waste sitting in the shipped tree. |
| 4. Non-brittle | 6 | `remote_ip.rs` and `secure_headers.rs` are careful, well-documented hand-rolled tower Services with real edge-case handling (e.g. `remote_ip.rs:171-177` correctly implements the MDN-recommended rightmost-trusted-proxy-skip algorithm). But `fallback.rs:39-41` ships a default that contradicts its own doc comment (see Evidence #2), and 9/16 live files have zero tests (see Evidence #4). |
| 5. Maintainable (DDD/OOP) | 7 | Each middleware is a cohesive type owning `name()/is_enabled()/config()/apply()` — a clean, well-named domain boundary per concern. `secure_headers.rs` cleanly separates preset data (JSON, `secure_headers.rs:28`) from behavior. |
| 6. Correctness | 4 | `compression.rs`, `timeout.rs`, `limit_payload.rs`, `powered_by.rs`, `format.rs`, `fallback.rs`, `logger.rs`, `static_assets.rs`, `static_assets_embedded.rs` have **zero** `#[test]`/`#[tokio::test]` (verified by grep, see Evidence #4). `fallback.rs:39-41` default-status-code bug (Evidence #2) would have been caught by even one test. `etag.rs` (the live, shipped-by-default caching middleware, `mod.rs:96-101`) also has zero tests. |
| 7. No reinvented wheels | 5 | Half the area is exemplary thin tower-http wrappers (cors, compression, timeout, catch_panic, limit_payload, static_assets, logger, powered_by) — genuinely fine, KPI not violated there. But `request_id.rs` hand-rolls exactly what `tower_http::request_id` (`SetRequestIdLayer`/`PropagateRequestIdLayer`, feature `request-id`) provides, and that feature isn't even enabled in `Cargo.toml:186-194`. `remote_ip.rs` reinvents what `axum-client-ip` does. See Library hypotheses. |
| **Overall** | **6** | Good bones (one trait, consistent shape, several excellent tower-http wrappers), pulled down by a dead-code file shipped in the tree, real copy-paste in the static-assets pair, a shipped default-value bug in fallback, and a glaring test-coverage gap across half the files. |

## Evidence log

1. **FACT**: `src/controller/middleware/_archive/content_etag.rs` (103 LOC) defines `EtagLayer`/`EtagMiddleware` that buffers the *entire response body* (`to_bytes(body, 5_000_000)`, line 72) and SHA-256-hashes it (`calculate_etag`, lines 99-101) to compute an ETag. It is **not** declared as a module anywhere: `src/controller/middleware/mod.rs:9-27` has no `mod _archive` or `mod content_etag`, and a repo-wide grep for `_archive` and `content_etag` outside this one file returns nothing. → **Judgment**: confirmed truly dead code shipped in the live source tree, superseded by the current `etag.rs` (introduced same commit `5bed78df`, later simplified to header-comparison-only in `14becefc`/`adebce86`). Should have been deleted rather than archived-in-place. → **KPI 1, 3** → **Severity: Medium** (no runtime risk since uncompiled, but visible patch-on-patch cruft directly named as a suspect).

2. **FACT**: `src/controller/middleware/fallback.rs:39-41` — `fn default_status_code() -> StatusCode { StatusCode::OK }`. The struct doc (`fallback.rs:19-21`) says "By default when enabled, returns a prebaked 404 not found page," and the `code` field doc (`fallback.rs:23-24`) says "the unlikely reason to return something different than `404`, you can set it here" — both implying the *default* should be 404. No example config in the repo (`examples/demo/config/*.yaml`) overrides `code`. `fallback.rs` has zero tests (see Evidence #4). → **Judgment**: default value contradicts documented and implied behavior; a user enabling `fallback: {enable: true}` with defaults gets an HTTP `200 OK` for what is semantically a "not found" catch-all response — a real, uncaught correctness bug. → **KPI 4, 6** → **Severity: High**.

3. **FACT**: `src/controller/middleware/static_assets.rs:24-76` (struct `StaticAssets` + `Default` impl + `default_must_exist`/`default_precompressed`/`default_fallback`/`default_folder_config` fns + `FolderConfig` struct) is byte-for-byte duplicated in `src/controller/middleware/static_assets_embedded.rs:28-79`, differing only by the absence of the `cache_control` field in the embedded variant (confirmed via `diff`). Both files are compiled under mutually-exclusive `#[cfg(feature = "embedded_assets")]` gates in `mod.rs:19-24`, so this isn't dead code, it's live duplication. → **Judgment**: this ~45-line config struct should be a single shared type (e.g. `common.rs` or defined once and `pub use`d) with the embedded/disk-serving logic as the only difference; instead the whole config surface was copy-pasted. → **KPI 2, 3** → **Severity: Medium**.

4. **FACT**: grepping every middleware file for `#[test]`/`#[tokio::test]`/`#[rstest]` shows **zero** test annotations in: `compression.rs`, `etag.rs`, `fallback.rs`, `format.rs`, `limit_payload.rs`, `logger.rs`, `powered_by.rs`, `static_assets.rs`, `static_assets_embedded.rs`, `timeout.rs`, `mod.rs` — 10 of 16 live files (verified by direct `grep -c` per file). Files that *do* have tests (`cors.rs`, `secure_headers.rs`, `remote_ip.rs`, `catch_panic.rs`, `request_id.rs`) are exactly the ones with the most hand-rolled logic, but `etag.rs` — the one middleware **enabled by default** in production (`mod.rs:96-101`, `etag::Etag { enable: true }`) — has none. → **Judgment**: coverage is inconsistent and skips the highest-risk, default-on middleware. → **KPI 6** → **Severity: High**.

5. **FACT**: `src/controller/middleware/request_id.rs:80-114` hand-rolls request-ID generation/propagation via `axum::middleware::from_fn` (25+ lines: regex sanitization at line 26, UUID fallback at line 113, manual header round-trip at lines 88-95), duplicating exactly what `tower_http::request_id::{SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestUuid}` provides out of the box. `Cargo.toml:186-194` enables `tower-http` features `trace, catch-panic, timeout, add-extension, cors, fs, set-header, compression-full` but **not** `request-id`. → **Judgment**: reinvented wheel; tower-http's request-id support is a near drop-in replacement for the generate-if-absent + propagate-to-response-header behavior implemented here. → **KPI 7** → **Severity: Medium**.

6. **FACT**: `mod.rs:74` carries `#[allow(clippy::unnecessary_lazy_evaluations)]` directly above `default_middleware_stack`. → **Judgment**: a lint suppression on the central middleware-assembly function is a minor smell per the rubric's "patch-on-patch" checklist (an `#[allow]` masking a real code-shape issue rather than fixing it) but is low-impact cosmetically. → **KPI 1** → **Severity: Low**.

## Patch-on-patch smells

- Archived-but-shipped dead code: `src/controller/middleware/_archive/content_etag.rs` (Evidence #1).
- Copy-pasted config struct across two features: `static_assets.rs:24-76` / `static_assets_embedded.rs:28-79` (Evidence #3).
- `#[allow(clippy::unnecessary_lazy_evaluations)]` suppression: `mod.rs:74` (Evidence #6).
- Leftover `// Corrected import` comment in dead code, itself evidence of an unresolved in-flight edit: `_archive/content_etag.rs:15`.
- TODO comment describing unimplemented etag-splitting design left in the dead file: `_archive/content_etag.rs:56-66` (harmless since uncompiled, but reinforces that this file was mid-refactor when abandoned).
- Doc/default mismatch in `fallback.rs:19-41` (Evidence #2) — a "temporary"-feeling default that was never reconciled with the doc comment.

## Library hypotheses

1. **Hand-rolled**: `src/controller/middleware/request_id.rs:80-114` (generate/sanitize/propagate `x-request-id`).
   **Candidate crate**: `tower_http::request_id` (already-present dependency `tower-http`, just needs the `request-id` Cargo feature turned on in `Cargo.toml:186-194`).
   **Why it might be simpler**: `SetRequestIdLayer` + `PropagateRequestIdLayer` + a custom `MakeRequestId` (for the Rails-style sanitization) replace the manual `axum::middleware::from_fn` function and remove the need to reason about header round-tripping by hand.
   **Risk / why it might not fit**: Loco's sanitization behavior (strip non-`\w-@` chars, 255-char cap, mimic Rails `ActionDispatch::RequestId`, `request_id.rs:100-113`) needs a custom `MakeRequestId` impl anyway, so the LOC savings may be modest (perhaps 30-40%, not a full replacement). **NEEDS SPIKE.**

2. **Hand-rolled**: `src/controller/middleware/remote_ip.rs:128-178, 242-299` (full tower `Layer`/`Service` for X-Forwarded-For trusted-proxy resolution).
   **Candidate crate**: `axum-client-ip` (purpose-built for exactly this: trusted-proxy-aware client IP extraction for axum).
   **Why it might be simpler**: removes ~150 lines of hand-rolled `Service`/`Layer` boilerplate (`remote_ip.rs:209-299`) and the custom IP-network trust-list parsing.
   **Risk / why it might not fit**: Loco's implementation has specific, documented parity goals with Rails' `remote_ip` middleware (`remote_ip.rs:75-94`) including exact selection semantics (rightmost-untrusted, not leftmost) and a `RemoteIP::{Forwarded,Socket,None}` enum consumers rely on (`remote_ip.rs:180-197`) — swapping crates risks behavior/API drift for existing Loco apps. **NEEDS SPIKE.**

3. **Hand-rolled**: `src/controller/middleware/secure_headers.rs:155-223` (custom tower `Service` applying a static header preset).
   **Candidate crate**: none identified as clearly simpler — this is a thin, well-tested ~30-line `Service` over a JSON preset file; a crate like `axum-helmet` would add a dependency for marginal gain over what's already minimal. **Not flagged** — this is a case where hand-rolling is appropriate (rubric guidance: don't recommend a swap for marginal gain).

## What is genuinely excellent (cited)

- `mod.rs:44-70` — the `MiddlewareLayer` trait is a clean, minimal abstraction (4 methods) that every one of the 12 middlewares implements uniformly; the doc comment above it (`mod.rs:36-43`) is a genuinely useful "checklist" for contributors adding new middleware.
- `cors.rs:74-159` — `Cors::cors()` cleanly maps a serde-friendly config (`Vec<String>` allow-lists) onto `tower_http::cors::CorsLayer`, handles the `Any` wildcard case distinctly from explicit lists, and is backed by real integration tests exercising actual header output via `insta::assert_debug_snapshot!` (`cors.rs:198-261`, `cors.rs:263-310`).
- `remote_ip.rs:55-94` — exceptional doc comment explaining the security tradeoffs of remote-IP inference, explicitly warning users when NOT to use it, and citing prior art (Rails, MDN spec) it deliberately diverges from.
- `secure_headers.rs:119-153` — clean separation of static preset data (`secure_headers.json`, loaded once via `OnceLock`, `secure_headers.rs:25-31`) from override-merge logic, with `BTreeMap` giving deterministic header ordering.
- `catch_panic.rs`, `compression.rs`, `timeout.rs`, `limit_payload.rs`, `powered_by.rs` — genuinely minimal, correct config wrappers over tower-http/axum primitives; no reinvention, no bloat. This is more than half the area and represents Loco's stated "lean" philosophy well.

## Top 3 things that would most raise the area's quality

1. **Delete `_archive/content_etag.rs`** outright (git history already preserves it) — zero reason to ship dead code in the tree; also fix `fallback.rs:39-41`'s default status code to `StatusCode::NOT_FOUND` (or explicitly document/rename the field if `OK` is intentional) since it currently contradicts its own doc comment and ships un-tested.
2. **Add tests for the 10 currently-untested files**, prioritizing `etag.rs` (enabled by default in production) and `fallback.rs` (has the default-value bug above) — even minimal `oneshot`-style request/response assertions matching the pattern already used in `cors.rs`/`catch_panic.rs`/`secure_headers.rs` would close the biggest correctness gap in the area.
3. **Unify `StaticAssets`/`FolderConfig`** between `static_assets.rs` and `static_assets_embedded.rs` into one shared config module, with only the serving backend (`ServeDir` vs embedded map) differing — removing ~45 duplicated lines and one future source of the two variants drifting apart.
