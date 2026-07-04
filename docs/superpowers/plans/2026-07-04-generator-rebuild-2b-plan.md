# Generator Rebuild 2b — Resource Scaffold Emission (Implementation Plan)

> **For agentic workers:** governed subagent execution. Opus governs (verifies each gate,
> commits); Sonnet implements one task at a time, sequentially, NO git writes. Steps are
> `- [ ]`. Grounding: `docs/superpowers/specs/2026-07-04-generator-rebuild-design.md` (stage
> 2b) + `examples/reference_spa/` (the exact target) + `loco-gen/src/column.rs` (2a, the
> single source of truth already built and tested).

**Goal:** Rewire `loco-gen`'s resource scaffold (`generate scaffold <Name> <fields…>`) to
consume the 2a `Column` model and emit the `reference_spa` per-resource shape: Rust DTOs
(ts-rs) + JSON-CRUD controller + TanStack Query hooks + React pages, deleting `mappings.json`,
`infer::parse_field_type`, the html/htmx flavors, and the `tera_ext` form-field string-matcher.

**Architecture:** `scaffold.rs` parses each `(field, spec)` via `column::parse_column` into a
`Column`, then builds ONE rich JSON context (per-resource names + per-column view-models) that
drives every `.t` template. Migration/model emission keeps the `migrate → db entities`
round-trip (entity stays introspected); DTO/controller/frontend derive from the `Column`s.

**Tech Stack:** Rust/rrgen/tera `.t` templates; ts-rs 12; React 19 + react-router v8 +
TanStack Query v5 (target output only — not built here).

## Global Constraints
- Local commits only (Jondot does push/PR/publish). Commit trailer:
  `Claude-Session: https://claude.ai/code/session_01W29Z6GystejksPaawG6AaS`.
- Governor verifies every gate firsthand and commits when no agent is mid-run. Implementers
  run sequentially and perform NO git writes.
- Test invocation: `cargo test -p loco-gen --lib` (integration `tests/*.rs` are RED from the
  inherited sea-orm E0053 baseline — out of scope). Snapshots via `cargo insta` / `INSTA_*`.
- DSL preserved (non-breaking surface): `name:type` nullable, `!` required, `^` unique+required,
  `references[?][:fk]`, `enum:a,b,c`, `decimal_len:P:S`, `array:inner`. Parsing is `column.rs`.

---

## DECIDED — canonical generator output (divergences from the hand-written reference)

The reference was hand-authored with three app-specific choices a *general* generator should
not hardcode. The generator emits the canonical form below; **2d regenerates and re-freezes
`reference_spa` to match** (the reference pins the stack + conventions, not byte-frozen prose).

1. **Enum types are resource-prefixed.** Column `status:enum:draft,published,archived!` on
   resource `Post` ⇒ Rust/TS type `PostStatus` (NOT `Status`). Rule:
   `{ResourcePascalSingular}{ColumnPascalSingular}`. Rationale: ts-rs exports flat into
   `frontend/src/bindings/`, so an unprefixed `Status` collides across resources. `column.rs`
   `dto_rust_type()` returns the bare name for enums; the scaffold layer overrides it with the
   prefixed name and generates the enum definition under that name.
2. **DTO = `id` + user columns + `created_at` + `updated_at`; Create/Update = all user
   columns.** (Reference hand-excluded `published_at` from Create/Update and omitted
   `updated_at` from the DTO.) `id`/`created_at`/`updated_at` are synthesized (never user
   fields) and never appear in Create/Update.
3. **`list` paginates only** (`ListParams { page, per_page }`); no per-column filters. (Reference
   had a bespoke `status` filter.) `api/<name>.ts` `List<Name>Params = { page?, per_page? }`.

Everything else matches the reference verbatim (client.ts, auth/, common.rs are once-per-app =
workstream 2c; here we emit only the per-resource files).

## Context schema (built in `scaffold.rs`, consumed by every `.t`)

```jsonc
{
  "pkg_name": "reference_spa",
  "name": "post",                 // raw resource arg
  "pascal_singular": "Post",      // PostDto, CreatePost, UpdatePost, PostStatus
  "snake_singular": "post",       // var names
  "snake_plural": "posts",        // file names, routes, entity module, query keys, url path
  "columns": [                    // USER columns only, in DSL order
    {
      "name": "status",
      "col_type": "String",           // Column::col_type()  (migration)
      "rust_type": "PostStatus",      // DTO field type (enum → resource-prefixed)
      "ts_override": null,            // Column::ts_type(); enum → null (native union)
      "nullable": false,
      "is_reference": false, "ref_target": null, "ref_fk": null,
      "is_enum": true,
      "enum_type": "PostStatus",
      "enum_variants": [ {"variant":"Draft","value":"draft"}, … ],
      "form_kind": "select",          // text|textarea|number|checkbox|datetime|select
      "set_expr": "params.status.as_str().to_string()"  // ActiveModel Set(...) rhs
    }, …
  ]
}
```

- `ts_override`: for non-enum columns pass `Column::ts_type()` (e.g. `"number"`, `"string"`,
  `"string | null"`, `"unknown"`); the DTO template emits `#[ts(type = "…")]` only when present.
- `set_expr`: enum (non-null) ⇒ `params.<name>.as_str().to_string()`; nullable enum ⇒
  `params.<name>.map(|v| v.as_str().to_string())`; else ⇒ `params.<name>`.
- `references`: a reference column has `is_reference=true`, `rust_type="i64"`, `ts_override="number"`,
  `col_type="BigInteger"`, and contributes to the migration `references` array as
  `(ref_target, ref_fk_or_empty)`. Its DTO/form field name is `<target>_id` (matching the
  introspected entity FK column).

---

### Task 2b-1: Column-driven migration + model plumbing

Make migration/model emission consume `column.rs` and delete the `mappings.json`/`infer`
field-type path from `model.rs`/`migration.rs`/`scaffold.rs`. Keep `model/model.t` output shape
identical (so only *values* change: `int`→`Integer`, FK→`BigInteger`), so this task's blast
radius is the migration snapshots only.

**Files:**
- Modify: `loco-gen/src/model.rs` — replace `get_columns_and_references` internals to parse via
  `column::parse_column`; return the SAME `(Vec<(String,String)>, Vec<(String,String)>)` shape
  (`(col_name, col_type_expr)` from `Column::col_type()`; `(ref_target, ref_fk)` for references,
  preserving the `"{fname}?"` nullable-ref marker the template/entity round-trip relies on — or
  drop the marker only if `model.t` no longer needs it; verify against current `model.t`).
- Modify: `loco-gen/src/migration.rs` — unchanged logic; it already calls
  `get_columns_and_references`. Confirm add/remove/reference/join paths still compile.
- Modify: `loco-gen/src/scaffold.rs` — replace its inline `parse_field_type` column loop with a
  new `columns_from_fields(fields) -> Result<Vec<column::Column>>` helper (shared, in `column.rs`
  or `scaffold.rs`), used to build the 2b-2/2b-3 context. FK columns emit `BigInteger` (kills
  the `i32` hardcode at `scaffold.rs:37-52`).
- Keep `infer::guess_migration_type` (migration-name inference) — only the `parse_field_type` /
  `FieldType` enum is being retired. Remove `parse_field_type` + `FieldType` once no caller
  remains (may slip to 2b-4).

**Steps:**
- [ ] Add `columns_from_fields` + a `partition_columns_refs(&[Column]) ->
      (Vec<(String,String)>, Vec<(String,String)>)` producing the migration template tuples via
      `Column::col_type()`. Unit-test: `title:string!` → `("title","String")`;
      `user:references` → ref `("user","")`, col_type `BigInteger`; `int` → `Integer`;
      `big_int` → `BigInteger`; `price:decimal!` → `Decimal`.
- [ ] Rewire `model.rs`/`scaffold.rs`/`migration.rs` to the new helpers; delete their
      `get_mappings()`/`parse_field_type` usage.
- [ ] `cargo build -p loco-gen` clean; `cargo test -p loco-gen --lib` — review + accept the
      migration snapshot deltas (int→Integer, FK i32→BigInteger, decimal etc.). Confirm deltas
      are ONLY those intended.
- [ ] **GATE (governor):** snapshots reviewed line-by-line for intended-only changes;
      `cargo clippy -p loco-gen --lib -- -D warnings` clean. Commit.

### Task 2b-2: DTO + controller templates (API scaffold)

Emit `src/dtos/<plural>.rs` + rewrite `src/controllers/<plural>.rs` to the reference shape;
build the rich context in `scaffold.rs`.

**Files:**
- Modify: `loco-gen/src/scaffold.rs` — build the full context schema above (per-resource name
  forms via `heck`/`cruet` per `infer.rs`'s inflection note: `cruet` for plural/singular,
  `heck` for case; per-column view-models incl. `enum_type`, `enum_variants`, `form_kind`,
  `set_expr`, resource-prefixed enum rust_type).
- Create: `loco-gen/src/templates/scaffold/api/dto.t` — emits `dtos/<plural>.rs`:
  - one `pub enum {enum_type}` per enum column (`#[derive(… TS)]`, `#[ts(export, export_to =
    "../frontend/src/bindings/")]`, `#[serde(rename_all = "snake_case")]`, variants from
    `enum_variants`, `impl From<String>`, `impl {enum_type} { pub fn as_str }`).
  - `PostDto` = `#[ts(type)]`-annotated `id:i64` + user cols + `created_at`/`updated_at`
    (`DateTimeWithTimeZone`, `#[ts(type="string")]`), `#[derive(Serialize,Deserialize,TS)]`.
  - `impl From<crate::models::_entities::<plural>::Model>` mapping (enum via `Type::from(m.x)`).
  - `Create<Pascal>` / `Update<Pascal>` = user cols only.
  - injections: `into: src/dtos/mod.rs, append: "pub mod <plural>;"`.
- Rewrite: `loco-gen/src/templates/scaffold/api/controller.t` — reference `posts.rs` shape:
  `ListParams{page,per_page}`, `not_found` ApiError helper, `list`/`get_one`/`create`/`update`/
  `remove` with `auth::JWT`, `Page<PostDto>`, `PostDto::from`, `set_expr` per column, routes
  `/api/<plural>`. injections: `controllers/mod.rs` append `pub mod <plural>;`; `app.rs` after
  `AppRoutes::` add `.add_route(controllers::<plural>::routes())`.
- Rewrite/trim: `loco-gen/src/templates/scaffold/api/test.t` — adjust to the new controller
  (or reduce to a compiling smoke; do not leave a stale htmx/html-shaped test).
- Modify: `loco-gen/src/scaffold.rs` — render `scaffold/api` with the new context; drop the
  `ScaffoldKind` match arms for Html/Htmx (see 2b-4) — for now keep Api arm.

**Steps:**
- [ ] Implement context builder; add focused unit tests for the tricky derivations:
      resource-prefixed `enum_type` (`Post`+`status`→`PostStatus`), `set_expr` for enum vs
      nullable-enum vs plain, `form_kind` mapping, reference field name `<target>_id`.
- [ ] Author `dto.t` + rewrite `controller.t`/`test.t`.
- [ ] Snapshot the `scaffold Post title:string! content:text! status:enum:draft,published,archived!
      price:decimal! published_at:tstz` output; diff the emitted `dtos/posts.rs` + controller
      against `examples/reference_spa` (allowing the DECIDED divergences: +`updated_at` in DTO,
      +`published_at` in Create/Update, no status filter).
- [ ] **GATE (governor):** emitted DTO+controller are byte-faithful to the reference modulo the
      three decided divergences; snapshots reviewed; `cargo test -p loco-gen --lib` + clippy
      clean. Commit.

### Task 2b-3: Frontend per-resource templates

Emit `frontend/src/api/<plural>.ts` + `frontend/src/pages/<plural>/{List,Show,New,Edit}.tsx` +
route injection.

**Files:**
- Create: `loco-gen/src/templates/scaffold/api/frontend_api.t` → `frontend/src/api/<plural>.ts`
  (TanStack hooks + `postKeys`, `List<Pascal>Params { page?, per_page? }`, typed by bindings).
- Create: `.../frontend_list.t`, `frontend_show.t`, `frontend_new.t`, `frontend_edit.t` →
  `frontend/src/pages/<plural>/{List,Show,New,Edit}.tsx`. Forms iterate `columns` using
  `form_kind` (text→`<input type=text>`, textarea→`<textarea>`, number→`<input type=text>` for
  Decimal-as-string else number, checkbox→`<input type=checkbox>`, datetime→
  `<input type=datetime-local>`, select→`<select>` over the enum's TS union). List table shows
  user columns; Show renders all DTO fields.
- Route injection: `.../frontend_route.t` (no body file; an injection-only template) OR fold
  injections into `frontend_list.t` — inject into `frontend/src/routes.tsx`: the four page
  imports and the four `RequireAuth` child routes. Use stable anchors already present in the
  2c base `routes.tsx` (add `// scaffold-imports` / `// scaffold-routes` markers there in 2c;
  for THIS task, inject relative to `createBrowserRouter([` / the `RequireAuth` children array —
  confirm anchors exist or add them to the reference to validate).
- Modify: `loco-gen/src/scaffold.rs` — render these under the Api arm.

**Steps:**
- [ ] Author the five templates + route injection, driven by the 2b-2 context (`form_kind`,
      `enum_type`, `ts_override` to decide string-vs-number inputs).
- [ ] Snapshot the emitted TS; diff against `examples/reference_spa/frontend/src/{api,pages}`
      (modulo decided divergences: New/Edit gain a `published_at` field; no status filter in
      the api hook).
- [ ] **GATE (governor):** emitted TS faithful to reference modulo divergences; snapshots
      reviewed; `cargo test -p loco-gen --lib` + clippy clean. Commit. (Real `tsc`/`vite build`
      happens in 2d against a full generated app.)

### Task 2b-4: Delete old flavors + single-flavor cleanup

Remove the html/htmx scaffold+controller flavors, `mappings.json`, the retired parser, and the
`tera_ext` form-field matcher; collapse `ScaffoldKind` to a single API flavor across loco-gen
AND the `src/cli.rs` surface.

**Files:**
- Delete: `loco-gen/src/mappings.json`; `loco-gen/src/templates/scaffold/{html,htmx}/**`;
  `loco-gen/src/templates/controller/{html,htmx}/**`; all their snapshots under
  `loco-gen/src/snapshots/`.
- Modify: `loco-gen/src/lib.rs` — remove `Mappings`/`RustType`/`FieldType`/`get_mappings`/
  `MAPPINGS`; remove `ScaffoldKind::{Html,Htmx}` (make scaffold always-API — either delete the
  enum and the `kind` field on `Component::Scaffold`/`Controller`, or reduce `ScaffoldKind` to a
  single `Api`). Update the mapping-based `mod tests` in lib.rs.
- Modify: `loco-gen/src/controller.rs` — collapse to the API controller only.
- Modify: `loco-gen/src/tera_ext.rs` — delete `FormField`/`ViewField` + their snapshot tests
  (they served html/htmx only). Keep the module only if still registering something; else drop
  `tera_ext` and its `new()` registration (the SPA scaffold needs no custom tera functions —
  confirm and simplify `new_generator()` accordingly).
- Modify: `loco-gen/src/infer.rs` — remove `parse_field_type` + `FieldType` (keep
  `guess_migration_type` + `MigrationType`).
- Modify: `src/cli.rs` — drop `--html`/`--htmx`/`--api` scaffold-kind flags + `kind` args (lines
  ~280-293, 315-327, 409-455); `Scaffold`/`Controller` always emit the SPA flavor. Update the
  `generate override` help text referencing `scaffold/htmx` (lines ~363-367, 1331-1343).
- Modify: docs/help strings mentioning html/htmx scaffolds if they break compilation.

**Steps:**
- [ ] Delete files + snapshots; strip the code; update both crates.
- [ ] `cargo build` (workspace: `-p loco-gen -p loco-rs`) clean; `cargo test -p loco-gen --lib`
      green with NO orphan/undeleted snapshots (run `cargo insta test --unreferenced=reject` or
      grep for leftover html/htmx snapshots); `cargo clippy --workspace --lib -- -D warnings`.
- [ ] **GATE (governor):** no `mappings.json`/`FieldType`/`ScaffoldKind::Html|Htmx`/`tera_ext`
      form-field references remain anywhere (grep clean); tests green; clippy clean. Commit.

## Self-Review notes
- Spec coverage: 2b-1 = migration/model plumbing; 2b-2 = DTO+controller; 2b-3 = frontend; 2b-4 =
  deletions/single-flavor. Matches design-spec stage-2b bullet list + the reference emitted-map.
- The three DECIDED divergences are the only intended differences from `reference_spa`; 2d
  regenerates + re-freezes the reference and updates its README/memory to note them.
- Inflection: follow `infer.rs` note — `cruet` plural/singular, `heck` case. Enum type name =
  `heck::to_upper_camel_case(cruet::to_singular(resource)) + heck::to_upper_camel_case(column)`.
- Risk: the `db entities` round-trip at scaffold time needs a DB (sqlite) — unchanged from today;
  real end-to-end regen is 2d's job, gated separately.
