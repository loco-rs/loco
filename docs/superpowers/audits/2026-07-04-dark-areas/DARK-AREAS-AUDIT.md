# Loco "Dark Areas" Audit — 2026-07-04

Four subsystems the maintainer flagged from gut memory as never-nailed / patch-on-patch.
Method: 4 parallel deep-reading agents (2 passes each, `file:line` citations, web-checked
2026 norms), then **governor verification of every load-bearing claim first-hand** + a compile
spike. Findings below are what survived verification; agent overstatements were corrected and are
noted as such.

Standing constraints: local commits only (Jondot does all push/PR/publish); 1.0 publish gated on
Sea-ORM 2.0.0 stable; breaking/feature/value churn is Jondot's call; give decisive recs + surface
the real forks.

---

## DECISIONS — 2026-07-04 (confirmed with Jondot)

- **Frontend (flagship) = React SPA + typed API client.** Competitive frame: Loco must compete with
  Claude spinning up Next.js+React apps → the default must be **mainstream React Claude is trained on**,
  as-lazy-as-`create-next-app`, agent-operable, and **NOT a Next.js RSC/server-component blend**.
  Stack: server = Loco JSON API; client = **Vite + React + react-router + TanStack Query**. Typed
  contract = **`ts-rs` derive (NOT OpenAPI)** — generator emits Rust DTOs with `#[derive(TS)]`
  (exported `.ts` bindings) + a typed fetch client + Query hooks consuming them. Rationale
  (spike-validated, see below): `ts-rs` gives the cleanest TS for the "non-vanilla" cases (rich
  enums → native discriminated unions; generics → real TS generics, not utoipa's mangled/inlined
  `Page_Post`), is structurally unable to lie about DTO shapes (derived from the same struct serde
  serializes — no `#[utoipa::path]` annotation to drift), and is a one-step cargo-native pipeline.
  **OpenAPI/utoipa is DEFERRED** to an optional *generated* (never hand-annotated) artifact for the
  `--api` headless/mobile/3rd-party story, gated by a schemathesis-style fidelity check if relied on.
  The server-rendered **html + htmx scaffold flavors are REMOVED**; Tera stays for mailers (+ optional
  server views, but not a scaffold flavor). `--api`-only remains the headless path. See memory
  `loco-frontend-competitive-frame`.

  **Spike evidence (scratchpad, 2026-07-04):** built the same "hard" API (i64, DateTime, Decimal,
  Option/nullable, an internally-tagged enum-with-data, a generic `Page<T>`, an error envelope) two
  ways. utoipa 5.5 → openapi-typescript 7.13: consuming TS is clean for vanilla + tagged enums
  (folklore partly outdated), BUT firsthand hit real footguns — `IntoParams` defaulted query params
  to `in=path, required=true` (wrong spec until `#[into_params(parameter_in = Query)]`), the `bearer`
  security scheme was referenced-but-undefined (needs a manual `Modify`), and the generic came out as
  a name-mangled `Page_Post` with the inner `Post` **inlined/duplicated** (DRY lost); research adds
  the decisive flaw — `responses(body=X)` is hand-declared and uncompiled vs the handler, so it can
  silently lie (documented prod near-miss). `ts-rs` 12 on the identical structs: `Attachment` → a
  clean native discriminated union, `Page<T>` → a **real TS generic** with `Post` reused via imports,
  can't drift. (Both need a 1-line i64 decision: openapi-ts→`number` lossy; ts-rs→`bigint` runtime-
  mismatched — generator picks `number` for ids.)
- **Generators = full single-source-of-truth rebuild.** One typed decision per column → migration +
  entity + DTO/props; DB introspection kept only as a fallback for non-DSL/foreign schema. Subsumes
  the FK/`i32` scaffold compile bug fix.
- **Features = collapse `bg_*` + trim.** Fold `bg_redis`/`bg_pg`/`bg_sqlt` into one `worker` feature
  (runtime-selected backend); target default `auth_jwt, cli, with-db, cache_inmem, worker`; drop dead
  `integration_test`. Breaking.
- **Correctness harness = deferred** until gen+frontend reshape templates, then a pairwise-N
  `cargo check` matrix (Postgres+Redis services) + the heavy end-to-end anchors.

### Build sequence
1. **Golden reference stack (spike, de-risks everything):** hand-build ONE app — Loco API + `utoipa`
   OpenAPI + Vite/React/react-router + generated typed client + TanStack Query — and prove it
   compiles + runs end-to-end (Vite dev proxy `/api` → Loco; prod static serve + SPA fallback). This
   becomes the generator's target output.
2. **Generator rebuild:** single-source-of-truth type system; emit models/migrations/entities + the
   API scaffold (OpenAPI-annotated) + React pages/hooks bound to the typed client.
3. **Feature-matrix consolidation** (`bg_*` → `worker`) — semi-independent, can run in parallel.
4. **Correctness harness** built against the new shape.

Interim: the currently-RED `loco-gen` snapshot suite on `release/0.17.0` is subsumed by step 2;
green it quickly on its own only if you want the branch clean before the rebuild lands.

---

## AREA 1 — Generator infrastructure (`loco-gen`)

### Verified mechanics
- A field type is decided by **4 parallel, independently-maintained tables** that agree only by
  string convention: `mappings.json` `rust` + `schema` + `col_type` (+nested array-inner map), and
  a **4th external** table — `sea-orm-cli generate entity`'s own DB→Rust introspection map, which is
  what *actually* types the entity struct. Only `ColType`↔`ArrayColType` (in `src/schema.rs`) is
  compiler-checked; the rest is "generate, then hope it compiles."
- Adding one new scalar type = **4 mandatory edit sites** (`mappings.json` rows, `ColType` variants
  `src/schema.rs:161-284`, `ColType::to_def()` arms `:326-470`, a `tera_ext.rs` form-field arm) +
  conditional schema helper + externally-unenforced sea-orm-cli agreement + regenerated snapshots.
- **`rrgen` is NOT a second template engine** — it's a thin file-write/inject wrapper over Tera
  (renders the whole `.t` incl. frontmatter in one pass, then string-splits). The genuine "double
  templating" is *temporal*: generated `.html` files carry `{% raw %}`-protected Tera that is
  rendered again at the target app's runtime by its own `TeraView`.

### CONFIRMED BUG (spike-verified) — ships a non-compiling app
- `scaffold.rs:38-52` hardcodes `i32`/`"Integer"` for all four `references` variants → `Params { pub
  user_id: i32 }`. But since 0.17's 64-bit-PK change, FK columns are created as `BigInteger`
  (`schema.rs:679-681`), so the introspected entity field is `i64`. `controller.t` emits
  `item.user_id = Set(self.user_id)` → **`i32` into `ActiveValue<i64>` = compile error**. Any
  `scaffold ... x:references` produces a broken app.
- **`cargo test -p loco-gen --test mod` is RED now (7 failures)** — templates were migrated to
  `Path<i64>` but snapshots + `scaffold.rs` FK typing were not. Trap: `insta accept` would green the
  suite while still shipping broken codegen. Correct fix: `scaffold.rs` refs → `i64`, then regen.

### Other verified hazards
- Arity check is skipped for the zero-param case (`tags:array` with no inner type parses as plain
  `Type("array")`, bypasses arity, emits a bare fn-item instead of a `ColType` → opaque build error).
- `col_type` dual-contract: array col_types are lowercase strings consumed as snake_case assoc-fns;
  scalars are PascalCase enum variants — one field name, two language mechanisms, undocumented.
- `int`→`i64`/`BigInteger` means true 32-bit `ColType::Integer` is **unreachable** from the DSL.
- `^` (uniq) is hard-coupled to NOT NULL (no "unique but nullable"); param *values* aren't validated
  (only count); null/uniq/required are combinatorial rows for scalars but an ad-hoc branch for refs.

### Rebuild options
- **A — Typed Rust type-registry** replacing `mappings.json` (one impl per type derives all four
  targets; nullability/uniqueness become orthogonal wrappers). Kills the casing/arity/edit-site class.
- **B — Single-derivation (RECOMMENDED core):** derive migration + entity + DTO from **one** type
  decision in loco-gen; keep `db entities` introspection as a *fallback* for non-DSL/foreign schema
  rather than the sole source of truth. Removes the structural cause of the FK bug by construction.
- **C — Minimal:** JSON→typed-Rust match, keep two-pipeline. Cheap, doesn't fix DTO/entity drift.
- **D — Reuse the introspected entity's field types for the scaffold DTO** (parse generated entity
  via `syn`). Would have prevented the FK bug for free.
- Recommendation: **B, with D's reuse for DTOs and A's typed mechanics** as the vehicle.

---

## AREA 2 — "Is the generated app actually correct?"

### Verified
- Wizard combinatoric space = **3 (db) × 3 (bg) × 3 (assets) = 27**. **~6 combos (22%) are ever
  compiled anywhere; `Queue` and `Blocking` background modes compile in 0 combos; Postgres compiles
  in exactly 1 (gated on `DATABASE_URL`).** All compiled combos use `background=Async`.
- `loco-gen`/`loco-new` template tests assert **text (insta snapshots), not compilation.** A renamed
  `loco_rs::` symbol used only in an uncompiled branch (Queue worker, Blocking, PG entities, htmx
  scaffold) ships broken and no test catches it.
- The only core-lib safety net is `loco-rs-ci-sanity.yml`, whose header comment claims a `paths` gate
  that **is not in the `on:` block** (verified) — so it runs on every PR today (good), but "fixing" it
  to match the comment would delete core-lib PRs' sole generated-app compile check.

### Recommendation
- **Pairwise-9 `cargo check` matrix** (all-pairs reduction of the 27; PICT/ACTS-style) with Postgres +
  Redis services so Queue/Blocking actually link — breadth. **Keep** the 2 heavy end-to-end anchors
  (`db.rs` sqlite+pg schema flow; 2–3 `sanity` full build+test) — depth. Wire it to also trigger on
  core `src/**`. Prior art: cargo-generate-action, create-t3-app CI flags, Rails' text-vs-boot split.

---

## AREA 3 — Cargo feature matrix

### Verified
- **The `bg_pg`/`bg_sqlt` split is largely illusory for binary weight:** `sqlx` is declared
  `features=["json","postgres","chrono","sqlite"]` and `sea-orm` with both `sqlx-postgres`+`sqlx-sqlite`
  **unconditionally** (`Cargo.toml:159`, `:73`). Whenever any of `with-db`/`bg_pg`/`bg_sqlt` pulls
  `sqlx`, **both** DB drivers compile regardless. Only `bg_redis` removes a real separate crate
  (`dep:redis`). The three `bg_*` flags gate ~150 lines of thin wrapper, not driver weight.
- **`default` enables all three bg backends**, but the wizard only ever produces `kind: Redis`;
  `bg_pg`/`bg_sqlt` are reachable only by hand-editing YAML. Asymmetric with cache (correctly
  `cache_inmem` on, `cache_redis` off).
- **`integration_test` is dead** — zero `.rs` references anywhere (verified).
- `embedded_assets` is real in the lib but never wired into `loco new`, and silently embeds nothing
  (runtime 404s) when combined with a clientside/React app (no `assets/` dir exists to scan).
- `auth_jwt` also gates the non-JWT `ApiToken` extractor (misnamed coupling). `with-db`-off is a
  genuine, exercised opt-out. Ecosystem norm (sqlx, sea-orm, tokio): no bundled "all drivers/all
  features" default — Loco's all-three-bg default is an outlier vs its own deps.

### Recommendation (each breaking item = Jondot's call)
- Target default set: **`auth_jwt, cli, with-db, cache_inmem, bg_redis`.**
- Consider **collapsing `bg_*` into one `worker` feature** (runtime-selected backend) since the split
  buys ~nothing; and/or **wire pg/sqlite queue choices into the wizard** so the capability is
  discoverable. Drop `integration_test` (zero-risk). Optionally split/rename `auth_jwt` vs `auth`.

---

## AREA 4 — Frontend ("never nailed")

### Verified (agent's "byte-identical / zero hx-" claim was OVERSTATED — corrected here)
- `loco new` asset choice is a single mutually-exclusive `AssetsOption::{Serverside, Clientside,
  None}`. Default (`SaasServerSideRendering`) = Tera server views; `Clientside` = a React SPA
  (`base_template/frontend/`: Rsbuild + React 19 + Biome + TS) served as static files with SPA
  index fallback; dev = two servers, Rsbuild proxies `/api` → 5150 (works only by the `/api` prefix
  convention). Prod = single origin (clean).
- The React starter mounts a **splash page, not a working example of calling the Rust API** — no
  `fetch`, no query lib, **no typed client**. `embedded_assets` + Clientside = silent runtime 404.
- **htmx scaffold is a shallow, awkward integration** (not "decorative duplication"): `views.rs` +
  `view_list` are ~identical to html, but `view_create`/`view_edit` use real `hx-post`/`hx-put`/
  `hx-ext`, `base.t` adds the htmx CDN + a **hand-rolled 30-line `submitjson` JS shim**, and
  `controller.t` diverges (`Json<Params>`, `HX-Redirect`, put+patch). It does full-page redirect
  navigation, **not** htmx's actual value (fragment swaps) — and the JS shim exists only to fight the
  controller's own `Json<Params>` choice. Undogfooded (Loco's demo is pure API), untouched since
  ~v0.9, no CI compile check. Real cost ≈ 7 htmx templates + base shim + controller variant + snaps.
- Server Tera + Fluent i18n path is **genuinely good** (hot reload, tested, real abstraction).
- **Zero OpenAPI-generation deps in the workspace** (no utoipa/aide/okapi/apistos) — the typed-API
  gap is real. Community issue #130 ("frontend approach") has been open for years, undecided.

### 2026 options (web-checked)
| Option | Modern? | 1-person maint. | Rust fit | Notes |
|---|---|---|---|---|
| Rails-8 Hotwire/Turbo no-build | yes | low | weak (no Rust import-map/asset-pipeline crate) | bespoke build needed |
| Inertia (server-routed SPA) | yes/growing | med | conceptually great, **adapters immature/fragmented** (axum-inertia v0.9, ~31★) | not a safe default yet |
| htmx + Tera | yes | low *if idiomatic* | strong | Loco's current impl is the weak link, not the paradigm |
| **SPA + typed API (React/Rsbuild + utoipa OpenAPI → generated TS client)** | yes (mainstream) | med | strong (`utoipa`+`utoipa-axum` de facto) | closes Loco's one real gap |
| Full-stack Rust (Leptos/Dioxus) | emerging | high | perfect lang, immature ecosystem | not a boring default in 2026 |

### Recommendation
- **Kill htmx** (subtraction). **Keep server Tera (html/api)** as the "no-JS-toolchain" path. **Make
  SPA + typed-API the flagship** by adding `utoipa` OpenAPI + a generated TS client into the React
  starter, and make the starter actually call the API. Fix the `embedded_assets`+Clientside 404.
- Product/taste forks are Jondot's: React vs Svelte/Solid; Rsbuild vs Vite; Tera co-flagship vs
  secondary; how opinionated the OpenAPI wiring is; whether to invest in a first-class Inertia adapter.

---

## Cross-cutting note
Areas are largely independent → each is its own spec/plan when greenlit. Sequencing candidates:
(0) hotfix the FK/i32 scaffold bug + red snapshot suite on the release branch (time-sensitive);
(1) generator rebuild; (2) correctness harness (unblocks confidence for everything else);
(3) feature-matrix consolidation; (4) frontend direction (largest product decision).
