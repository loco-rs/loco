# Area: A2 · HTTP Core & Routing

## Scope (files reviewed, with LOC)

- `src/controller/routes.rs` (585)
- `src/controller/app_routes.rs` (458)
- `src/controller/format.rs` (655)
- `src/controller/mod.rs` (253)
- `src/controller/monitoring.rs` (460)
- `src/controller/describe.rs` (42)
- `src/controller/backtrace.rs` (67)
- `src/controller/extractor/mod.rs` (4)
- `src/controller/extractor/shared_store.rs` (29)

(`src/controller/extractor/auth.rs` and `validate.rs` are explicitly excluded — A8.)

Total in-scope LOC: 2,553. All files read in full. Cross-referenced against
`src/errors.rs` (Error enum) and `src/cli.rs` (consumer of route "actions") for
correctness context, and against `axum 0.8.9` source
(`~/.cargo/registry/.../axum-0.8.9/src/routing/method_routing.rs`) to verify a
brittleness hypothesis (confirmed by a standalone repro, see Evidence #1).

## Scores

| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 5 | `format.rs` implements the same response concepts twice with diverging logic (free fns vs `RenderBuilder`) — `format.rs:81-83` (`text`) vs `format.rs:289-297` (`RenderBuilder::text`); `mod.rs:184-227` matches `Self::WithBacktrace` twice in one function. |
| 2. Economy of concepts | 6 | Reasonable concept count (`Routes`/`Handler`/`AppRoutes`/`ListRoutes`/`RenderBuilder`/`Json`/`describe`) for a Rails-style routing DSL over Axum, but `describe.rs` is an entire module that exists only to reverse-engineer information the caller already had at `Routes::add` call sites (`routes.rs:81-89`). |
| 3. Low LOC | 6 | `format.rs` public surface is duplicated (module fns + builder) for text/html/json/redirect/view/template — six concepts, twice each (`format.rs:60-210` vs `format.rs:284-395`); `routes.rs:82` calls `describe::method_action(&method)` and discards the result, then calls it again on the next line. |
| 4. Non-brittle | 3 | `describe.rs:19-42` parses `format!("{method:?}")`, Axum's private `Debug` output, with a regex that uses `.captures()` (first match only) instead of `.captures_iter()` — confirmed by reproduction to silently drop all but the first HTTP method on a multi-verb route (`get(a).post(b)`); `mod.rs:220-227` always returns `400 Bad Request` for `WithBacktrace`, regardless of the wrapped error's real nature (e.g. a `serde_json::Error` during response serialization, which is a server bug, not a bad request). |
| 5. Maintainable (DDD) | 5 | `Routes`/`AppRoutes`/`Handler` are a cohesive, well-named domain model matching Rails' routing story cleanly (`routes.rs:9-20`, `app_routes.rs:24-34`). But the format.rs dual-API means every new response type must be added twice and kept in sync — already drifted: `yaml()` (`format.rs:159-164`) and `empty_json()` (`format.rs:119-121`) have no `RenderBuilder` equivalent. |
| 6. Correctness | 4 | Confirmed bug: `describe::method_action` under-reports HTTP methods for any multi-verb single-URI route, feeding wrong data into `cargo loco routes` (`cli.rs:1185`, `cli.rs:1209`) and the startup route log (`app_routes.rs:301`); no test exercises this path (see Evidence #1). Dead/duplicate call at `routes.rs:82`. Elsewhere, `monitoring.rs` has excellent success/failure test coverage across db/cache/queue permutations (`monitoring.rs:119-460`). |
| 7. No reinvented wheels | 4 | `describe.rs` hand-rolls introspection over Axum's *private* `MethodRouter` `Debug` impl instead of capturing methods at the call site where they're statically known (`routes.rs:81`, caller already wrote `get(handler)`/`post(handler)`); `RenderBuilder::json` (`format.rs:354-368`) hand-serializes into a `BytesMut` instead of reusing the `axum::Json`/`Json` wrapper already used two lines away by the free `json()` fn (`mod.rs:170-178`, `format.rs:110-112`). |
| **Overall** | **5** | Solid, well-documented public routing API (routes.rs/app_routes.rs) let down by a genuinely fragile, bug-confirmed introspection hack (describe.rs) and an internally-duplicated response-formatting API (format.rs) that has already drifted out of sync with itself. |

## Evidence log

1. **FACT** — `describe.rs:19-42`'s `method_action` runs
   `get_describe_method_action().captures(&method_str)` (singular `captures`,
   first-match-only) against `format!("{method:?}")`, the `Debug` output of
   Axum's private `MethodRouter`/`MethodEndpoint` types
   (`axum-0.8.9/src/routing/method_routing.rs:585-599`, `:1278-1284`).
   I reproduced this standalone (scratch crate, axum 0.8, same regex):
   for `let mr = get(h1).post(h2);`, `format!("{mr:?}")` produces
   `MethodRouter { get: BoxedHandler, ... post: BoxedHandler, ... }` — the
   regex with `.captures_iter()` finds **both** `get` and `post`, but the
   shipped code's `.captures()` only ever returns `get` (first match), silently
   dropping `post`. **Judgment**: this is a real, reproducible bug in an
   introspection feature, not a hypothetical. It affects `Handler.actions`
   (`routes.rs:85`) → `ListRoutes.actions` (`app_routes.rs:32`) → the
   `cargo loco routes` CLI output sort/display (`cli.rs:1185`, `cli.rs:1209`)
   and the per-route startup log line (`app_routes.rs:300-301`, via
   `ListRoutes`'s `Display`, `app_routes.rs:36-47`). Actual HTTP dispatch is
   unaffected (the real `MethodRouter` is still stored and used), only the
   *reported* method list is wrong. **KPI**: 4 (Non-brittle), 6 (Correctness),
   7 (reinvented wheel — hand-rolled and fragile where the info was already
   known at the call site). **Severity: HIGH.**

2. **FACT** — `routes.rs:81-89`:
   ```rust
   pub fn add(mut self, uri: &str, method: axum::routing::MethodRouter<AppContext>) -> Self {
       describe::method_action(&method);                 // <- result discarded
       self.handlers.push(Handler {
           uri: uri.to_owned(),
           actions: describe::method_action(&method),     // <- called again
           method,
       });
       self
   }
   ```
   **Judgment**: `describe::method_action` (a `format!` + regex call) is
   invoked twice per route registration; the first call's result is thrown
   away. Classic leftover-from-evolution artifact. **KPI**: 1 (holistic
   vision), 3 (LOC). **Severity: LOW-MEDIUM** (wasted cycles at boot only, but
   clear evidence of unreviewed patch-on-patch).

3. **FACT** — `mod.rs:180-253`, `impl IntoResponse for Error`: the arm
   `Self::WithBacktrace { inner, backtrace: _ }` is matched once at
   `mod.rs:185-194` (by reference, purely for `tracing::error!` side effects)
   and matched **again** at `mod.rs:220-227` (by value) to build the actual
   HTTP response — where it unconditionally returns
   `(StatusCode::BAD_REQUEST, ErrorDetail::with_reason("Bad Request"))`
   regardless of what `inner` actually is. Given `errors.rs:23-27`
   (`impl From<serde_json::Error> for Error { ... Self::JSON(val).bt() }`),
   any JSON (de)serialization failure — including a failure inside
   `RenderBuilder::json`'s own `serde_json::to_writer` (`format.rs:359`) —
   becomes `WithBacktrace` and is reported to the client as `400 Bad Request`,
   even when the failure is in the server's own response construction (should
   arguably be `500`). **Judgment**: silent-wrong status-code mapping for a
   whole class of internal errors. **KPI**: 4 (non-brittle), 6 (correctness).
   **Severity: MEDIUM.**

4. **FACT** — `format.rs` implements the same six response concepts twice:
   - `empty()` (`format.rs:60-62`) vs `RenderBuilder::empty()` (`format.rs:304-306`)
   - `text()` (`format.rs:81-83`, uses `String::into_response()`) vs
     `RenderBuilder::text()` (`format.rs:289-297`, manually sets
     `Content-Type: text/plain; charset=utf-8` and builds `Body::from`)
   - `html()` (`format.rs:139-141`, uses Axum's `Html` type) vs
     `RenderBuilder::html()` (`format.rs:339-347`, manually sets
     `Content-Type: text/html; charset=utf-8`)
   - `json()` (`format.rs:110-112`, delegates to the `Json` wrapper in
     `mod.rs:170-178`, which delegates to `axum::Json`) vs
     `RenderBuilder::json()` (`format.rs:354-368`, manually
     `serde_json::to_writer` into a `BytesMut::with_capacity(128)`)
   - `redirect()` (`format.rs:182-184`, uses Axum's `Redirect::to`) vs
     `RenderBuilder::redirect()`/`redirect_with_header_key()`
     (`format.rs:375-395`, manually builds `303 See Other` + `Location`/custom
     header)
   Two response concepts have **no** builder equivalent at all: `yaml()`
   (`format.rs:159-164`) and `empty_json()` (`format.rs:119-121`).
   **Judgment**: two parallel, independently-maintained implementations of the
   same domain concept ("build an HTTP response of kind X"), already
   inconsistent with each other in both behavior (different `Content-Type`
   construction paths) and surface area (missing methods). **KPI**: 1
   (holistic vision), 2 (economy of concepts), 3 (LOC), 5 (maintainability —
   any new response format now needs to remember to touch two places, and
   already forgot twice). **Severity: MEDIUM-HIGH** (655 LOC file, ~40% of it
   is this duplication + its mirrored test suite at `format.rs:409-655`).

5. **FACT** — `backtrace.rs:18-35`: a 17-line block of commented-out regex
   strings (`"^<?tokio"`, `"^<?future"`, `"^<?tower"`, ... `"^catch_unwind"`)
   sits inside the live `NAME_BLOCKLIST` initializer, never compiled.
   **Judgment**: dead code left in the tree, no comment explaining why it's
   disabled or whether it's a future TODO. **KPI**: 1 (holistic vision).
   **Severity: LOW** (harmless, but a named patch-on-patch smell in the
   rubric).

6. **FACT** — Three `#[allow(clippy::...)]` suppressions in scope:
   `mod.rs:182` (`#[allow(clippy::cognitive_complexity)]` on
   `IntoResponse for Error`), `app_routes.rs:277` (same lint, on `to_router`),
   `routes.rs:249` (`#[allow(clippy::needless_pass_by_value)]` on `layer`).
   **Judgment**: two independent functions in this small area needed a
   cognitive-complexity opt-out — a direct signal of functions that grew via
   incremental edits past what the linter considers healthy. **KPI**: 1
   (holistic vision). **Severity: LOW.**

7. **FACT** — `app_routes.rs:16-20` (`NORMALIZE_URL = Regex::new(r"/+")`) and
   its use in `collect()` (`app_routes.rs:66-101`, esp. `:80-91`) manually
   joins prefix + route URI with `"/"` and then regex-collapses repeated
   slashes and strips trailing slashes, rather than composing real nested
   `axum::Router` values (which normalize paths internally via its matcher).
   This is exercised by decent tests (`app_routes.rs:346-372`, multiple
   slash/prefix edge cases via `insta` snapshots), so it's *tested*
   brittleness, not silent brittleness — noted for KPI 7, not KPI 6.
   **Severity: LOW.**

## Patch-on-patch smells

- Dead discarded call: `routes.rs:82` (see Evidence #2).
- Commented-out code block: `backtrace.rs:18-35` (see Evidence #5).
- Duplicate/inconsistent implementations of the same concept: `format.rs`
  free-function API vs `RenderBuilder` API (see Evidence #4) — six concepts
  done twice, two of the six (`yaml`, `empty_json`) done only once, showing
  the duplication already drifted out of sync.
  handling now.
- `#[allow(clippy::cognitive_complexity)]` x2 (`mod.rs:182`,
  `app_routes.rs:277`) + `#[allow(clippy::needless_pass_by_value)]`
  (`routes.rs:249`) — lint suppressions as a proxy for organic complexity
  growth (see Evidence #6).
- Double `match self { Self::WithBacktrace ... }` pattern in one function
  (`mod.rs:184-227`, see Evidence #3) — the same variant matched twice for two
  different purposes (logging, then response-building) instead of being
  extracted once.

## Library hypotheses

1. **Hand-rolled code**: `describe.rs:19-42` — regex-scraping Axum's private
   `Debug` output to recover which HTTP methods a `MethodRouter` was built
   with.
   **Candidate**: none — this is not a "wrong tool" problem, it's a
   "reconstructing known-but-discarded information" problem. `routes.rs:81`'s
   caller (`Routes::add(uri, method)`) receives the `MethodRouter` *after* the
   caller already wrote `get(handler)`/`post(handler).put(handler2)` — the
   method set was known statically one line before it was erased into an
   opaque `MethodRouter`.
   **Why it might be simpler**: dropping the regex entirely in favor of
   capturing methods explicitly (e.g. `Routes::get/post/put(...)` helper
   methods, or accepting `&[Method]` alongside the `MethodRouter`) removes a
   whole module and a confirmed bug class, with no new dependency.
   **Risk / why it might not fit**: would change the public `Routes::add` API
   surface used throughout the ecosystem (generators, starters) — a breaking
   change. **NEEDS SPIKE** to scope the API-compat blast radius (loco-gen
   templates, starter apps).

2. **Hand-rolled code**: `RenderBuilder::json` (`format.rs:354-368`) — manual
   `BytesMut` + `serde_json::to_writer` JSON body construction, duplicating
   the free `json()` fn's simpler `Json(t).into_response()` path
   (`format.rs:110-112`, `mod.rs:170-178`).
   **Candidate**: no external crate needed — this is an intra-codebase reuse
   opportunity, not a library gap. `RenderBuilder::json` could call
   `axum::Json(item).into_response()` and merge its headers/body into the
   builder's `Builder`, exactly mirroring what the module-level `json()`
   already does.
   **Why it might be simpler**: removes ~10 LOC and one of the two JSON
   code paths that must be kept behaviorally identical.
   **Risk**: merging headers from an already-built `Response` into a
   `Builder` requires a small amount of restructuring since `RenderBuilder`
   is `Builder`-based, not `Response`-based. **NEEDS SPIKE**.

3. **Hand-rolled code**: `app_routes.rs:16-20`, `:80-91` — regex-based
   multi-slash collapsing + manual prefix/URI string joining for route
   normalization.
   **Candidate**: `axum::Router::nest`/`.merge()` composition (already a
   project dependency) building real nested `Router<AppContext>` values
   instead of a flat `Vec<Handler>` + string concatenation.
   **Why it might be simpler**: Axum's own router already normalizes nested
   paths without a hand-written regex.
   **Risk / why it might NOT fit**: Loco's `AppRoutes::collect()`
   (`app_routes.rs:66-101`) needs the *flat, introspectable* list of
   `(uri, methods)` for `cargo loco routes` and startup logging — a real
   `Router` doesn't expose its route table for iteration, so this refactor
   would need Axum to add introspection (it doesn't) or would reintroduce
   the very same "recover info from something opaque" problem as hypothesis
   #1. Likely **not a fit** — noted only for completeness. **NEEDS SPIKE**
   (low priority, likely negative result).

## What is genuinely excellent (cited)

- `routes.rs:22-359` (`Routes` builder: `add`/`merge`/`merge_all`/`prefix`/
  `layer`/`nest`) and `app_routes.rs:49-313` (`AppRoutes`) are thoroughly,
  usefully doc-commented with runnable `rustdoc` examples for every public
  method (e.g. `routes.rs:60-79`, `:91-145`, `:153-196`, `:205-231`,
  `:273-327`) — genuinely good API documentation practice, rare to see this
  consistently across an entire module.
- `app_routes.rs:283-299` — the comment block explaining Axum middleware
  ordering (LIFO via `app.layer`, why `ServiceBuilder` was deliberately
  avoided, with a linked rationale to a real `rust-lang/crates.io` PR) is an
  example of exactly the kind of institutional-knowledge documentation that
  prevents future patch-on-patch damage.
  care was taken.
- `monitoring.rs:100-460` — the test suite for `readiness()` covers
  success/failure permutations across DB (`:204-231`, `:233-260`), in-memory
  cache (`:263-295`), Redis cache (`:297-368`), and queue presence/absence
  (`:379-459`), each asserting both status code and JSON body. This is real
  behavior-exercising coverage, not happy-path smoke tests — a strong example
  of KPI 6 done right, in the same area that has the multi-method-route gap.
- `routes.rs:361-585` and `app_routes.rs:315-458` — `nest`/`merge`/
  `merge_all`/prefix-normalization edge cases (root path, trailing slash,
  missing leading slash, multiple merges then nest) are all unit-tested with
  clear assertions, giving real confidence in the URI-composition logic
  despite its hand-rolled string manipulation.

## Top 3 things that would most raise the area's quality

1. Fix `describe::method_action` (`describe.rs:22`) to use `captures_iter`
   instead of `captures`, add a regression test with a multi-verb route on one
   URI (`get(a).post(b)` at one `.add(...)` call) asserting both methods are
   reported — closes a confirmed, currently-silent correctness bug that
   propagates into user-facing `cargo loco routes` output.
2. Collapse `format.rs`'s dual API: make `RenderBuilder`'s `text`/`html`/
   `json`/`redirect` delegate to the same primitives the module-level
   functions use (or vice versa), and add the two missing builder methods
   (`yaml`, `empty_json`) or intentionally drop the asymmetry — one API
   surface for one concept, matching the rest of the area's quality bar.
3. Reconsider `mod.rs:220-227`'s blanket "`WithBacktrace` → 400 Bad Request"
   mapping — either match on the *inner* error to pick the right status, or
   document explicitly why all backtrace-wrapped errors are client-facing
   400s (currently true only by accident of which `From` impls call `.bt()`
   today, per `errors.rs:23-27`).
