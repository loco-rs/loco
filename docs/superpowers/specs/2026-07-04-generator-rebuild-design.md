# Generator Rebuild — Design Spec (Workstream 2)

Part of the 2026-07-04 "dark areas" program. Greenlit direction: **full single-source-of-truth
rebuild** of `loco-gen` so one typed column decision drives every output, the generator emits the
**reference stack** (`examples/reference_spa/`), and the shipped FK `i32`-vs-`i64` bug + the
`mappings.json` four-table drift class are eliminated by construction.

Grounding: area-1 audit (`docs/superpowers/audits/2026-07-04-dark-areas/DARK-AREAS-AUDIT.md`) +
the proven reference (`examples/reference_spa/README.md` — the "emitted-once vs per-resource" map).

## Problem being fixed (from the audit, re-confirmed first-hand)
- **Four parallel tables** in `mappings.json` (`rust`/`schema`/`col_type`/`arity` + array inner-map)
  that agree only by string convention; only `ColType` is compiler-checked.
- **Two independent type pipelines** (migration/entity vs scaffold DTO) → the FK `i32`(DTO) vs
  `i64`(entity) compile bug; the loco-gen template snapshot suite is currently RED from a half-done
  64-bit migration.
- **Combinatorial DSL rows:** `!`/`^` baked into type-name strings (triple rows per type) instead of
  orthogonal modifiers; arity checked only for the `:param` path (bare `array` bypasses it).
- **html/htmx/api three-way** scaffold flavors (htmx shallow/undogfooded) — removed per the frontend
  decision; single flavor = **JSON API + React/ts-rs SPA**.

## The single source of truth (the crux)

One Rust model, defined once, compiler-checked, deriving ALL outputs. Replaces `mappings.json` +
`infer.rs`'s string matching + `tera_ext`'s form-field string-matching.

```rust
// loco-gen/src/column.rs  (new)
pub enum ScalarType {
    String, Text, Uuid,
    SmallInt, Int, BigInt, SmallUnsigned, Unsigned, BigUnsigned,
    Float, Double, Decimal, DecimalLen { precision: u32, scale: u32 }, Money,
    Bool,
    Date, Time, DateTime, DateTimeTz,
    Json, Jsonb,
    Blob, VarBinary { len: u32 }, BinaryLen { len: u32 },
}

pub enum ColumnKind {
    Scalar(ScalarType),
    Array(ScalarType),                                   // pg array
    Reference { target: String, fk_field: Option<String> },  // ALWAYS i64 FK
}

pub struct Column { pub name: String, pub kind: ColumnKind, pub nullable: bool, pub unique: bool }
```

Every output is a single compiler-checked `match` (no parallel tables):

| Derivation | Signature | Consumers |
|---|---|---|
| Migration column | `fn col_type(&Column) -> loco_rs::schema::ColType` | migration `.t` |
| Wire/DTO Rust type | `fn dto_rust_type(&Column) -> String` (e.g. `Option<Decimal>`) | `dtos/<name>.rs` |
| ts-rs override | `fn ts_type(&Column) -> Option<String>` (`i64`→`"number"`, `Decimal`/dates→`"string"`, `Option`→`"… \| null"`) | `#[ts(type=…)]` |
| Form control | `fn form_input(&Column) -> FormKind` (Text/Number/Textarea/Select/Checkbox/DateTime) | React `New/Edit` |

- **Nullability/uniqueness are parameters, not rows.** `references` is ALWAYS `BigInt`/`i64` (kills
  the FK bug — DTO and migration both derive from the same `Column`).
- **DSL surface preserved** (familiar, non-breaking): `name:type` = nullable, `name:type!` = NOT NULL,
  `name:type^` = unique+NOT NULL, `name:references[?][:fk]`, `name:decimal_len:10:2`. Parsed
  **orthogonally**: strip suffix → flags; base name → `ScalarType`; `:params` → arity fields.
  Arity/param **values** validated at parse time (fixes the bare-`array` bypass + `decimal_len:abc`).
- Exhaustive unit tests: every `ScalarType` × {nullable, unique} → asserted `col_type`/`dto_rust`/
  `ts_type`/`form_input`. This is the compiler-checked replacement for the snapshot-only coverage.

## Entity: derive-consistent, keep introspection (design fork — recommended)

Full "emit the sea-orm entity from `Column`" (audit Option B) means hand-generating entities with
relations/`ActiveModelBehavior` — high-risk. **Recommended synthesis:** keep the working
`migrate → db entities` round-trip for the **entity**, but derive **migration + DTO + controller +
frontend + form** from the one `Column`. Consistency holds by construction: introspection reads back
exactly the `Column`-driven migration, so the entity type and the `Column`-derived DTO type agree —
which is precisely what the FK bug violated. (Full entity-emit stays a documented future option.)

## Emission (matches the reference exactly)

From `(resource_name, Vec<Column>)` the scaffold emits — per `examples/reference_spa`:
- **migration** (`create_table` with `ColType`s; refs = i64 FK)
- **model** + entity (via `db entities`)
- **`dtos/<name>.rs`** — `PostDto`/`Create`/`Update` with `#[derive(TS)]` + `ts_type` overrides +
  `From<Model>`; enum types for closed-set columns
- **`controllers/<name>.rs`** — JSON CRUD returning DTO shapes, `ApiError`, `auth::JWT`
- **`frontend/src/api/<name>.ts`** — TanStack Query hooks + keys
- **`frontend/src/pages/<name>/`** — List/Show/New/Edit typed by bindings
- **injections** — `app.rs` route, `controllers/mod`, `dtos/mod`; then the bindings export step
- Templates stay tera/rrgen `.t`, but driven by **one** JSON context derived from `Column` (not the
  four tables). `mappings.json`, the html/htmx templates+snapshots, `scaffold.rs`'s `i32` FK
  hardcode, and the `tera_ext` form-field string-matching are **deleted**.

## base_template (loco-new) — once-per-app
Replace the rsbuild/splash frontend with the reference's setup: Vite+React+react-router+TanStack
Query, `api/client.ts`, `auth/` (token/Login/RequireAuth), `dtos/common.rs` (`Page<T>`,`ApiError`),
the bindings export step, clientside serving config, and (pending) `TS_RS_EXPORT_DIR` pinned so
`export_to` is robust. (Not fixing the inherited sea-schema/E0282 baseline bugs — moot for 1.0.)

## Staging (each stage independently gated + committed)
- **2a — type model:** `column.rs` (`ScalarType`/`ColumnKind`/`Column`) + orthogonal DSL parser +
  the four derivations + exhaustive unit tests. No `mappings.json`. Foundational.
- **2b — resource scaffold:** rewrite model/migration/scaffold/controller generation to consume
  `Column` and emit the per-resource Rust+TS files above; new `.t` templates; delete the old
  flavors/mappings. Gate: generated files byte-match the reference's shapes; snapshots regenerated.
- **2c — app scaffold:** base_template rewrite for the once-per-app frontend + `dtos` infra.
- **2d — validation:** `loco new` + `generate scaffold post …` reproduces `reference_spa`; it builds
  (`cargo check` + `tsc` + `vite build`); ts bindings idempotent; 401/CRUD live smoke. (Seeds the
  deferred correctness harness — workstream 4.)

## DECIDED (2026-07-04)
- **Entity = keep db-introspection** (migrate → `db entities`); derive migration + DTO + controller +
  frontend + form from `Column`. Consistency by construction; FK bug fixed; no risky entity hand-emit.
- **Enums = first-class:** `status:enum:draft,published,archived` → `ScalarType::Enum(Vec<String>)`
  → Rust enum (serde snake_case) + ts-rs discriminated union + `<select>` form + validation. The
  headline demo of why we skipped OpenAPI.
- **`int` quirk fixed:** `int` → `Integer`/`i32` (intuitive), `bigint` → `BigInteger`/`i64`;
  ids/FKs/`references` stay `i64`. Deliberate breaking change for the clean 1.0 slate (the old
  `int`→`i64` made true 32-bit unreachable — audit hazard #1).

## Risks / open questions
1. **Blast radius:** this rewrites the crate the whole ecosystem's `generate` depends on; existing
   user scaffolds reference `ColType::*` names directly — keep those stable (they're unchanged).
4. **`db entities` dependency** at scaffold time (needs a DB) — the reference used sqlite; keep that
   requirement or gate a `--no-db` path?
