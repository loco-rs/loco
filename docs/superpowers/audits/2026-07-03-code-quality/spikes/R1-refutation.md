# R1 — Adversarial Refutation of Iteration-1 Findings

Verdicts: REAL (survives refutation), FALSE-POSITIVE (disproven), OVERSTATED (real but
lower severity/different scope than claimed).

---

## F1 — Redis 3s boot sleep — **REAL**

`src/bgworker/redis.rs:957-978`, `create_provider()`:

```rust
pub async fn create_provider(qcfg: &RedisQueueConfig) -> Result<Queue> {
    let client = connect(&qcfg.uri)?;
    let registry = JobRegistry::new();
    let token = CancellationToken::new();
    let run_opts = RunOpts { .. };
    debug!(..., "creating Redis queue provider");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;   // line 971
    Ok(Queue::Redis(client, ..., run_opts, token))
}
```

`connect()` (line 238-241) is `Client::open(url)` — this is `redis`-crate's **lazy**
client constructor; it does not open a TCP connection or perform any handshake. So the
sleep happens *before any connection to Redis has even been attempted* — it cannot be
"waiting for the connection to warm up" or "waiting for a consumer group to be created."
There is no `XGROUP CREATE`, no stream setup, no retry/backoff loop anywhere near this
function (`grep -n "sleep|XGROUP|consumer|stream" src/bgworker/redis.rs` — the only
other `sleep` hits are a per-worker polling-interval sleep at line 221, unrelated, and
one inside `#[cfg(test)]`).

This is called unconditionally from `src/bgworker/mod.rs:761`
(`Ok(Some(Arc::new(redis::create_provider(qcfg).await?)))`) — i.e. on **every real app
boot** that configures a Redis queue, not just in tests. `pg::create_provider` and
`sqlt::create_provider` have zero sleeps in their equivalent function bodies (all
`sleep` hits in `pg.rs`/`sqlt.rs` are inside `mod tests`) — confirming the pg/sqlt
asymmetry claim.

Git blame nails the origin: commit `aa79a5404` ("isolated redis pg tests (#1390)",
2025-04-23) added this line with comment `// Wait for 3 seconds to ensure Redis is
ready` in the *same commit* that introduced testcontainers-based Redis test setup
(`setup_redis_container`). This reads as test-warmup logic that leaked into the
production code path rather than being scoped to `#[cfg(test)]`. The comment has since
even been dropped from the source (current line 971 has no comment at all — bare,
unexplained `sleep`).

**Verdict: REAL.** Confirmed load-bearing-free, unconditional, every-boot 3s stall
specific to the Redis provider, provably added for test isolation and never gated out
of the production path.

---

## F2 — `reference_id()` normalized-vs-raw inconsistency — **REAL** (worse than described: reproduced with a concrete FK-name collision)

Call sites (`src/schema.rs`):
- `create_table_impl` (~line 648): `reference_id(&nz_from_table)` where
  `nz_from_table = normalize_table(from_tbl)` (line 644) — **normalized** (pluralized +
  snake_cased) input.
- `add_reference` (~line 780): `reference_id(totbl)` — **raw** input (the function
  separately computes `nz_totbl = normalize_table(totbl)` at line 777 but does not use
  it here).
- `remove_reference` (~line 865): same pattern, `reference_id(totbl)` on raw input.

```rust
fn normalize_table(table: &str) -> String { cruet::to_plural(table).to_snake_case() }
fn reference_id(totbl: &str) -> String {
    format!("{}_id", cruet::to_singular(totbl).to_snake_case())
}
```

For **regular** English nouns this looks harmless — `to_singular` is idempotent enough
that `reference_id("user")`, `reference_id("users")`, and
`reference_id(normalize_table("user"))` all reduce to `"user_id"`. I verified this by
vendoring the exact `cruet = "1.0"` dependency (per `Cargo.toml:100`) and running both
code paths against 19 test nouns.

But for **irregular plurals**, the two call sites genuinely diverge, because
`cruet::to_plural` correctly maps `person → people` / `child → children`, while
`cruet::to_singular` does **not** reverse this (it returns `people`/`children`
unchanged instead of reducing back to `person`/`child`):

```
raw='person' → direct reference_id("person")            = "person_id"
raw='person' → reference_id(normalize_table("person"))   = reference_id("people") = "people_id"   <-- DIFFERENT
raw='child'  → direct reference_id("child")              = "child_id"
raw='child'  → reference_id(normalize_table("child"))     = reference_id("children") = "children_id" <-- DIFFERENT
```

Concretely: a migration with `create_table!` declaring a `refs` entry `("person", "")`
(the doc comment at schema.rs:664-666 explicitly shows this convention: `// user, None
// users, None // user, admin_id` — i.e. singular relation names are the documented
input) produces column **`people_id`** and constraint
`fk-{table}-people_id-to-people`. A later migration calling
`add_reference(m, "table", "person", "")` (the natural, same-identifier way a developer
would write it, consistent with schema.rs's own `/// person -> people, movies ->
movie` comment at line 707) computes column **`person_id`** and constraint
`fk-{table}-person_id-to-people` — a different column name and a different constraint
name from what `create_table_impl` actually created. `remove_reference` would then try
to `DROP CONSTRAINT fk-{table}-person_id-to-people`, which never existed, and fail (or,
worse, silently no-op on SQLite where `remove_reference` does nothing at all for FKs).

This is a genuine, reproducible, irregular-noun-triggered bug rooted in (a) calling
`reference_id` on differently-shaped inputs (pre-normalized-plural vs. raw) at the two
sites, compounded by (b) `cruet::to_singular` not being a true inverse of
`cruet::to_plural`. Root-caused fix: `add_reference`/`remove_reference` should call
`reference_id(&nz_totbl)` (the value they already compute) to at least match
`create_table_impl`'s convention — though note this only changes *which* wrong answer
you get, since the underlying `cruet` irregular-plural gap persists either way and
independently produces a wrong (over-pluralized) FK column name for tables like
`people`/`children`.

**Verdict: REAL**, reproduced with a runnable minimal repro against the actual vendored
`cruet` version pinned in `Cargo.toml`.

---

## F3 — Error→HTTP collapses 20+ variants to 500 — **REAL**

`src/errors.rs:32-151` defines the `Error` enum with **35 variants** (some
`cfg`-gated): `WithBacktrace, Message, QueueProviderMissing, TaskNotFound, Scheduler,
Axum, Tera, JSON, JsonRejection, YAMLFile, YAML, EmailSender, Smtp, Worker, IO, DB,
ParseAddress, Unauthorized, NotFound, BadRequest, CustomError, InternalServerError,
InvalidHeaderValue, InvalidHeaderName, InvalidMethod, Model, Redis, Sqlx, Storage,
Cache, Generators, VersionCheck, Any, Validation, AxumFormRejection`.

`src/controller/mod.rs:204-249`, the `match self { ... }` that builds
`public_facing_error`, has exactly **7** named arms: `NotFound` (404), `Unauthorized`
(401), `CustomError` (caller-supplied), `WithBacktrace` (400), `BadRequest` (400),
`JsonRejection` (`err.status()`, dynamic), `Validation` (400) — everything else
(**28 variants**) falls to the catch-all at line 245-248:

```rust
_ => (StatusCode::INTERNAL_SERVER_ERROR, ErrorDetail::new("internal_server_error", "Internal Server Error")),
```

This is not merely theoretical over-collapse of genuinely-server-side errors — it
misclassifies real client/expected conditions:

1. **`AxumFormRejection`** (`src/errors.rs:150`, `#[from] axum::extract::rejection::FormRejection`)
   is structurally identical to `JsonRejection` (a request-body-rejection carrying its
   own correct status code via `.status()`), but unlike `JsonRejection` it has **no**
   matching arm, so malformed `Form<T>` extraction — a pure client error — becomes a
   500.

2. **`Model(ModelError)`** wraps `ModelError::EntityNotFound` (`src/model/mod.rs:17-18`),
   a "not found" condition that is semantically identical to `Error::NotFound`, yet
   `Error::Model(_)` has no arm and collapses to 500. This is not hypothetical: in
   loco's own shipped example, `examples/demo/src/controllers/auth.rs:162-164`:
   ```rust
   async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
       let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
       format::json(CurrentResponse::new(&user))
   }
   ```
   If the JWT's `pid` no longer resolves to a user (deleted/revoked account — an
   entirely normal, expected condition for a `/auth/current` endpoint), `find_by_pid`
   returns `Err(ModelError::EntityNotFound)`, which `?` converts to
   `Error::Model(...)`, which becomes **500 Internal Server Error** instead of 404 (or
   401). Contrast with the officially generated CRUD scaffold
   (`loco-gen/src/templates/scaffold/api/controller.t:49-51`), which manually avoids
   this trap: `item.ok_or_else(|| Error::NotFound)` — i.e. the framework's own
   generator authors clearly know `ModelError`/`Model` propagation is a footgun and
   route around it by hand in generated code, but don't fix the `IntoResponse` mapping
   itself, and the framework's own hand-written demo controller (`current`, not
   generator output) falls straight into the trap.

So the count (~7 explicit vs. ~28 collapsed) is accurate, and at least two of the
collapsed variants (`AxumFormRejection`, `Model`) are demonstrably user-facing/expected
error conditions, not genuine server faults — this is a real correctness gap, not just
an aesthetic one.

**Verdict: REAL** (finding's variant count and severity both hold up).

---

## F4 — `describe.rs` verb-drop bug — **REAL**, scope is diagnostic-only (routes listing), not live routing

`src/controller/describe.rs:10-12,19-24`:
```rust
fn get_describe_method_action() -> &'static Regex {
    DESCRIBE_METHOD_ACTION.get_or_init(|| Regex::new(r"\b(\w+):\s*(BoxedHandler|Route)\b").unwrap())
}
pub fn method_action(method: &MethodRouter<AppContext>) -> Vec<http::Method> {
    let method_str = format!("{method:?}");
    get_describe_method_action()
        .captures(&method_str)   // <-- first match only
        ...
}
```
No `captures_iter`/loop exists anywhere in the file, and the only caller
(`src/controller/routes.rs:82,85`) calls this once per `Handler` and uses the result
directly as `actions: Vec<http::Method>` — nothing downstream re-scans for additional
verbs.

axum 0.8.9's `Debug` impl for `MethodRouter`
(`axum-0.8.9/src/routing/method_routing.rs:587-598`) unconditionally prints **every**
verb field via `debug_struct`, in this fixed order: `get, head, delete, options, patch,
post, put, trace, connect, fallback, allow_header`. For a router built with a single
verb (e.g. `get(handler)`), only one field renders as `BoxedHandler`/`Route` and the
rest are `None` — `.captures()` finds the one true match, so the common
one-verb-per-`add()` convention (as literally used by every generated route, e.g.
`loco-gen/src/templates/scaffold/api/controller.t:96-99`:
`.add("/", get(list)).add("/", post(add))` — **two separate `add()` calls**, not one
chained multi-verb router) is unaffected.

However, axum explicitly supports and documents chaining verbs on a single
`MethodRouter` (`axum::routing::get(handler).post(other_handler)` — shown in axum's own
doc example at `method_routing.rs` `on()` docs), and nothing in Loco's `Routes::add`
signature (`fn add(mut self, uri: &str, method: axum::routing::MethodRouter<AppContext>)`,
`src/controller/routes.rs:81`) prevents a user from passing such a chained
multi-verb router. If they do, the Debug string contains two matches (e.g. `get:
BoxedHandler(..), ..., post: BoxedHandler(..)`) and `.captures()` returns only the
first (`get`, since it's first in field-declaration order) — the `post` verb is
silently dropped from the reported `actions`.

Impact is scoped to whatever consumes `Handler.actions` for description purposes
(`cargo loco routes` / admin route-listing), not actual request dispatch — axum's
router still dispatches both verbs correctly regardless of what `describe.rs` reports.
So this is a real, reproducible bug in the *description/introspection* path, not a
routing-correctness bug; the original finding was already scoped to `describe.rs`
specifically and didn't claim broken dispatch, so it is not overstated.

**Verdict: REAL** (as scoped by the reviewer — diagnostic/description output only).

---

## F5 — ViewEngine `Infallible`-but-panics — **REAL**, not merely theoretical

`src/controller/views/mod.rs:69-96`:
```rust
impl<S, E> FromRequestParts<S> for ViewEngine<E> {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> ... {
        let Extension(tl): Extension<Self> = Extension::from_request_parts(parts, state)
            .await
            .expect("TeraLayer missing. Is the TeraLayer installed?");
        Ok(tl)
    }
}
```

The `Extension<ViewEngine<E>>` layer is **not** installed by any unconditional
framework boot step. It is only added by an app-level, opt-in `Initializer`:
`examples/demo/src/initializers/view_engine.rs:44` and the identical
`loco-new/base_template/src/initializers/view_engine.rs:44`
(`router.layer(Extension(ViewEngine::from(tera_engine)))`), invoked from
`Hooks::initializers()`. The **default** trait implementation is empty
(`src/app.rs:395`: `async fn initializers(...) -> Result<...> { Ok(vec![]) }`), and
nothing in `src/boot.rs`/`src/app.rs`/`src/controller/routes.rs` unconditionally
inserts this extension.

Whether a scaffolded app even gets the `view_engine` initializer file at all is gated
by a wizard choice: `loco-new/src/settings.rs:80-81` only sets
`initializers: Some(Initializers { view_engine: true })` when
`AssetsOption::Serverside` was picked; `loco-new/src/generator/mod.rs:91` conditionally
emits the initializer file on that flag; `loco-new/tests/templates/initializers.rs`
confirms the file (and its `pub mod view_engine;` registration) is entirely absent
otherwise. So: an API-only-scaffolded app, or any app where a developer prunes/edits
their `initializers()` Vec, or a handler mistakenly written to accept `ViewEngine<E>`
without the initializer wired up, hits this `.expect()` panic on the very first request
to that route — despite the type system's `Rejection = Infallible` promising this
can't happen.

No test in the repo boots a live app and dispatches an HTTP request through a
`ViewEngine`-extracting handler to catch this class of regression (`views/engine.rs`
tests only construct `TeraView` directly, never via `FromRequestParts`). This exact gap
is independently corroborated by iteration-1's own `A4-views.md` finding, and
`docs-site/content/docs/how-to/render-views.md:74` documents the literal panic message
as a known failure mode for users to watch out for — i.e. it's a documented, real,
user-hitting footgun, not a theoretical one. Introduced verbatim (Infallible + expect,
unexplained) in the original template-rendering-stack commit and never revisited since.

**Verdict: REAL.** Severity: moderate — gated behind a scaffolding choice or a
maintenance mistake rather than triggerable on every default boot, but concretely
reachable in supported configurations (API-only project + later hand-added Tera view
route; or initializer accidentally removed), and it is a hard panic (process/task
crash on that request), not a graceful error response.

---

## Summary

| # | Verdict | One-liner |
|---|---------|-----------|
| F1 | REAL | Unconditional 3s `sleep` in `redis.rs:971` fires before any Redis connection is attempted (`connect()` is lazy `Client::open`), on every real boot (`bgworker/mod.rs:761`), added in a test-isolation commit and never gated to `#[cfg(test)]`; pg/sqlt have zero equivalent. |
| F2 | REAL | Reproduced with vendored `cruet 1.0`: irregular nouns (`person`/`people`, `child`/`children`) make `create_table_impl`'s `reference_id(&nz_from_table)` (schema.rs:648) and `add_reference`/`remove_reference`'s `reference_id(totbl)` (schema.rs:780,865) yield genuinely different FK column/constraint names (`person_id` vs `people_id`), because `cruet::to_singular` isn't a true inverse of `cruet::to_plural`. |
| F3 | REAL | 7 of 35 `Error` variants get distinct HTTP status (`controller/mod.rs:204-249`); 28 collapse to 500 via the `_ =>` arm, including `AxumFormRejection` (client error, no arm unlike sibling `JsonRejection`) and `Model(ModelError::EntityNotFound)` — the latter demonstrably fires as a wrongful 500 in loco's own `examples/demo/src/controllers/auth.rs:162-164` `current()` handler. |
| F4 | REAL | `describe.rs:23` uses `.captures()` (first-match-only) against axum 0.8.9's `MethodRouter` Debug output, which lists all 9 verb fields in fixed order; a router built via axum's documented `get(h1).post(h2)` chaining yields two regex matches but only the first (`get`) is kept — silently drops `post` from the reported route `actions`. Scope is limited to the description/introspection path (e.g. `cargo loco routes`), not live dispatch. |
| F5 | REAL | `ViewEngine`'s `Rejection = Infallible` (`views/mod.rs:74`) is contradicted by an internal `.expect()` panic (`:82`) whenever `Extension<ViewEngine<E>>` wasn't layered in; that layering is an opt-in `Initializer` (`examples/demo/src/initializers/view_engine.rs:44`) gated by a scaffolding wizard choice (`loco-new/src/settings.rs:80-81`), not unconditionally installed by boot (`src/app.rs:395` default `initializers()` is empty) — reachable in supported configurations, no test exercises the live-request path. |
