# Loco Rails-Lens Rewrite Audit (2026-07-04)

Whole-codebase re-evaluation through Rails' mode of operation, triggered by the
bgworker/ActiveJob episode: when a rewrite target has ONE genuinely-hard sub-part,
the abstraction usually lives ABOVE it — separate "clean the interface" from "unify the
hard impl detail." Method: 10 parallel Rails-grounded analysis agents, one per
subsystem, each mapping Loco → its Rails analog, scoring candidates, flagging
uncertainty, and required to argue the strongest case AGAINST each rewrite.

Scores are 1-5. Breaking: 1=none .. 5=severe. LOC direction is stated honestly
(an interface win can ADD lines — bgworker was +189).

Standing constraints: local commits only; Sonnet implements, Opus governs; 1.0 publish
gated on Sea-ORM 2.0.0 stable; breaking/value churn is Jondot's call — crystallized below.

---

## The headline: most of Loco is already Rails-clean

The single most important finding is how much did NOT survive scrutiny. Loco already has
clean adapter traits (`CacheDriver`, `StoreDriver`, `StorageStrategy`, `Mailer`,
`ViewRenderer`, `MiddlewareLayer`, `Task`, `Hooks`), and several "obvious" rewrite
targets are myths:

**Confirmed NON-candidates (prove-why-not WON):**
- **`cli.rs` is not a bloated dispatch match.** Dispatch is ~180 lines of thin arms;
  context is booted once and threaded through. The 1364 lines are two self-contained
  tree-printers (~290) + ~300 lines of clap `after_help`. A Thor-style `Command` trait
  registry is a FALSE analogy — clap's `#[derive(Subcommand)]` already IS the declarative
  registry, with compile-time exhaustiveness + `#[cfg]` gating a `dyn` vec would forfeit
  (+150-300 LOC for strictly worse guarantees). REJECT unless third-party plugin commands
  become a roadmap goal (the only condition that flips it).
- **`schema.rs` vs `db/schema.rs` are NOT duplicated.** One is the migration DDL builder,
  the other runtime introspection/dump. Disjoint jobs, shared filename. No merge.
- **`ColType` enum is not bloat** — its variant names are a code-generator contract
  (`loco-gen` writes them verbatim into migrations). Irreducible; reconcile only with the
  Sea-ORM 2.0 bump.
- **No `Model` trait, by design** — adding one would shadow Sea-ORM's `EntityTrait`.
- **`ConditionBuilder`** is the Rails-flavored query surface; removing it exposes raw
  `sea_query`. Keep.
- **OpenDAL driver-collapse already happened** — 1 real `StoreDriver` (OpenDAL) + Null +
  five thin typed constructors. Not a candidate.
- **Generic `Valid<E>` validate extractor** — regresses call-site ergonomics, doesn't cover
  the opaque tier cleanly, breaks a public API for surface the macros already collapsed. REJECT.
- **`auth.rs` "892 LOC extractor"** — 67% tests; production ~297 lines, already config-driven
  (no type-per-location family to collapse).
- **`loco-new`/`loco-gen` crate separation** and **`setup.rhai`** — deliberate,
  Rails-faithful (install-time decoupling; `template.rb`-style scripting). Leave alone.
- **Mailer render-in-job** — blocked by `include_dir!`'s compile-time `Dir` handle.
- **`Hooks` god-trait split / `BootResult` reshape** — Loco's central ergonomic identity,
  bound `<H: Hooks>` through boot/cli/scheduler/db. Breaking=4-5, wide generic ripple, mostly
  aesthetic payoff. OUT OF SCOPE; documented as known smells.

---

## Real bugs surfaced (independent of any rewrite)

These are defects, not refactors — worth fixing regardless of the program scope.

| # | Bug | Location | Severity |
|---|---|---|---|
| B1 | `backup.rs` writes to secondaries **sequentially**; `mirror.rs` does the same op **concurrently** — same concept, drifted behavior | `storage/strategies/backup.rs` (5× inline loop) vs `mirror.rs:266` | Perf/consistency |
| B2 | MySQL apps can migrate/connect but **silently cannot seed or dump tables** (`BackendNotSupported` inconsistently); `dump_schema` handles MySQL, `get_tables`/seed introspection don't | `db/seed.rs`, `db/schema.rs`, `db/connect.rs` | Latent capability gap |
| B3 | Custom Tera filters registered in only 2 of 4 construction sites → `number_with_delimiter` etc. **silently no-op in mailer templates and inline `format::template`** | `tera.rs:6`, `views/mod.rs:59` (missing) vs `engine.rs`/`engine_embedded.rs` (present) | Correctness |
| B4 | `handle_job_command` re-creates a second `AppContext` — the lone CLI arm that re-boots instead of reusing the threaded context | `cli.rs:1178` | Coherence/waste |
| B5 | `add()` calls `describe::method_action` twice, discarding the first result (regex + `{:?}` runs 2× per route) | `controller/routes.rs:82` | Dead work (free fix) |

---

## Ranked rewrite program

### TIER 1 — Greenlight recommended (clear wins, worth the churn)

#### T1.1 — Unify storage `backup` + `mirror` into one `ReplicatedStrategy`
- **Rails analog:** ActiveStorage has ONE `MirrorService`. Loco split it into two ~identical strategies.
- **Shape:** `backup.rs` (1244) + `mirror.rs` (964) share the identical "primary must succeed →
  fan out to secondaries → collect errors → policy decides" skeleton across 5 ops. Only real
  differences: read-fallback (a bool) and failure-policy richness (mirror's enum is a strict
  SUBSET of backup's). Merge → `ReplicatedStrategy { primary, secondaries, failure_policy,
  read_from_secondaries }` + one `FailurePolicy` + one concurrent `fan_out_to_secondaries`
  combinator (promote `mirror.rs`'s existing helper). Keep `::mirror()`/`::backup()` constructor
  fns for intent-naming. Fixes B1 in passing.
- **Scores:** Impact=4 Effort=3 Risk=2 **Breaking=3** | **net-LOC ≈ −300 shipped**, conf med
- **Churn to decide:** merges two public `FailureMode` enums + two public strategy types into one.
  Zero internal callers; external-only. 0.17 is already breaking → marginal cost near-zero.
- **Non-breaking fallback (T1.1a):** if you defer the merge, still hoist the concurrent combinator
  into `backup.rs` — −70 LOC, fixes B1, Breaking=0.

#### T1.2 — `AppContext`: add `#[non_exhaustive]` + a builder
- **Rails analog:** `Rails.application` is a stable façade; adding an internal component doesn't break apps.
- **Shape:** `AppContext` has all-`pub` fields and is NOT `#[non_exhaustive]` (unlike `errors.rs`,
  which is). Every future subsystem field is therefore a breaking change by construction. Add
  `#[non_exhaustive]` + `AppContext::builder()` (keep field reads for `FromRef`/`State` ergonomics).
- **Scores:** Impact=4 Effort=2 Risk=2 **Breaking=3** | net-LOC ≈ +25, conf med
- **Churn to decide:** one-time break to direct struct-literal construction (mostly test code) NOW,
  to make all FUTURE additions non-breaking. 0.17 is the cheap window. **Highest impact-per-risk in the audit.**

#### T1.3 — Unify Tera construction into one factory (fixes B3)
- **Rails analog:** ActionView + ActionMailer share one rendering stack; a helper registered once is everywhere.
- **Shape:** One `tera::instance()`/`render_string()` that always registers filters, consumed by all
  four sites. Split "core vs app filters" so config/env YAML rendering stays filter-free.
- **Scores:** Impact=4 Effort=2 Risk=2 **Breaking=2** | net-LOC ≈ −15, conf med
- **Churn to decide:** templates that currently error on an unknown filter would start succeeding
  (a fix, but a semantics shift).

#### T1.4 — Consolidate `extra_db` into `multi_db`
- **Rails analog:** first-class named databases in one config.
- **Shape:** `extra_db` is literally `multi_db` with n=1 — three pieces (`MultiDb` + two initializers)
  implement one concept. Delete `extra_db`; keep one named-connections abstraction. NOT Sea-ORM-2.0-sensitive.
- **Scores:** Impact=4 Effort=2 Risk=2 **Breaking=4** | net-LOC ≈ −35, conf med
- **Churn to decide:** removes a public initializer (`ExtraDbInitializer`) — breaking for anyone using it.

### TIER 2 — Good, smaller or additive (batch these)

#### T2.1 — Middleware: Rails-style `insert_before`/`swap`/`remove` builder (ADDITIVE) + dedup registry
- Additive user-facing win (the Rails `config.middleware` gap), non-breaking, +50 LOC. Pair with the
  LOC-negative dedup of the three-parallel-lists registry (−40) and the copied `static_assets` config (−60).
- **Scores:** Impact=3-4 Effort=2-3 Risk=1-2 Breaking=1-2 | net-LOC ≈ −50 combined
- **Note:** verify real demand for reordering — config `enable` toggles already cover the common case.

#### T2.2 — `loco-gen`: data-drive `render_form_field` (+ `GenerateResults::merge`)
- One 250-line `match rust_type` of ad-hoc HTML `format!`s → a `rust_type → InputSpec` table +
  3-4 renderers. 80+ insta snapshots guard it (low risk). Internal-only.
- **Scores:** Impact=3 Effort=3 Risk=2 Breaking=1 | net-LOC ≈ −90+ 
  , conf med

#### T2.3 — Mailer: add `deliver_now` alongside `deliver_later`
- Rails-parity gap; Loco only has `deliver_later` semantics today. Additive, trivial. Verify bgworker
  modes don't already give an implicit sync path first.
- **Scores:** Impact=3 Effort=1 Risk=1 Breaking=1 | net-LOC ≈ +15

#### T2.4 — Auth/testing small safe wins
- `OptionalJWT` extractor (interface GAP — verify axum's `Option<T>` blanket doesn't already cover it),
  `map_auth_model_error` helper, delete dead `#[async_trait]` in `testing/db.rs`. Ship the last two now.
- **Scores:** Impact=2-3 Effort=1 Risk=1 Breaking=1 | net-LOC ≈ mixed small

#### T2.5 — Coherence sweep (bugs B4, B5 + small dedup)
- Fix `handle_job_command` double-boot (B4); delete the double `method_action` call (B5); dedup the
  `Start` block across the two `main`s and extract the `StartMode` ladder; `paginate`/`fetch_page`
  dedup in the query builder.
- **Scores:** Impact=2 Effort=1-2 Risk=1-2 Breaking=1 | net-LOC ≈ −70 combined

### TIER 3 — Design calls / defer (crystallized, NOT auto-greenlit)

| Item | Why deferred |
|---|---|
| **B2 MySQL seed/dump gap** | Real bug, but fixing = deciding whether MySQL is a supported backend (scope). Fix as a bug OR document PG/SQLite-only. **Your call on MySQL support intent.** |
| **Routing: retire `describe.rs` regex** | The verb-explicit builders exist but `.add(uri, get(h))` is the ecosystem default and the only way to pass a pre-composed multi-verb router. Retiring the regex is a `.add`-deprecation MIGRATION, not a swap. Breaking=4. |
| **Validate naming inversion + opaque tier** | The lossy empty-body variant has the short name; the good field-errors default has the verbose `WithMessage` suffix. Fixing is pure breaking rename churn; needs usage data on the opaque tier. |
| **Testing request-fn explosion → builder** | 4 request fns × boot variants → one builder. Real smell, but blast radius = EVERY app's test suite; needs a codemod. Hold for the breaking wave. |
| **Scheduler subprocess-per-cron-tick** | Re-boots the whole app per tick; incoherent with bgworker/task. But subprocess isolation is a defensible `whenever`-style choice and `shell:true` jobs block a clean bgworker merge. Design discussion. |
| **`Hooks::boot` default via associated `Migrator` type** | Removes a required do-nothing method, but cfg-gating an associated type is awkward and Breaking=5. Medium confidence it's clean. |
| **Hooks god-trait split / BootResult reshape** | Out of scope — blast radius unjustified by aesthetic payoff. |

---

## Honest aggregate

If TIER 1 + TIER 2 ship: roughly **−400 to −500 shipped LOC**, three real bugs fixed (B1, B3, B4/B5),
one latent bug decided (B2), plus two additive Rails-parity wins (`insert_before`, `deliver_now`) and
the future-proofing `#[non_exhaustive]`. No Sea-ORM-2.0-gated code is touched except where explicitly noted.

The bgworker lesson held up as a filter: it correctly GREENLIT the storage merge (interface over a
non-hard part) and correctly REJECTED the cli/validate/schema/ColType rewrites (the "duplication" was
either a contract, disjoint, or already collapsed). The net is a much smaller, sharper program than a
naive "rewrite everything" pass — which is the point.

---

## GREENLIT (2026-07-04, Jondot)

**Scope:** TIER 1 + TIER 2 (all 9 items). **B2 (MySQL):** document PG/SQLite-only —
make unsupported-backend errors uniform + explicit, no new backend maintained.
TIER 3 stays deferred. Sonnet implements sequentially, Opus governs/gates/commits; local
commits only; atomic commit per item; gate = fmt + clippy `-D warnings` (all backend combos)
+ targeted tests + zero snapshot drift.

**Execution order (isolated/safe first → breaking/large → additive):**
1. T2.5 coherence sweep + bugs B4, B5 (+ Start dedup, StartMode ladder, paginate/fetch_page)
2. T1.3 Tera factory unify (fixes B3)
3. B2 document PG/SQLite-only (uniform unsupported-backend errors)
4. T1.4 `extra_db` → `multi_db` consolidation (breaking: drops `ExtraDbInitializer`)
5. T1.2 `AppContext` `#[non_exhaustive]` + builder (breaking: struct-literal construction)
6. T1.1 storage `ReplicatedStrategy` merge (breaking: two public types→one; fixes B1)
7. T2.1 middleware `insert_before`/`swap`/`remove` (additive) + registry/static-assets dedup
8. T2.2 `loco-gen` `render_form_field` data-drive + `GenerateResults::merge`
9. T2.3 mailer `deliver_now` (verify bgworker sync path first)
10. T2.4 auth/testing small wins (`OptionalJWT` — verify axum `Option<T>` blanket first;
    `map_auth_model_error`; delete dead `#[async_trait]`)

CHANGELOG updated per breaking item with the crystallized behavior delta.
