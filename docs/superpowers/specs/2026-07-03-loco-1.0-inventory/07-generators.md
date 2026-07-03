# 07 — Code Generators (loco-gen)

Verified from `loco-gen/src/{lib.rs,mappings.json}` and `src/cli.rs` (main-loop read, not agent).

## Generator kinds (`Component` enum, loco-gen/src/lib.rs:237; CLI `ComponentArg`, src/cli.rs:175)

`cargo loco generate <kind>` — kinds:

| Kind | CLI | Notes | file:line |
|------|-----|-------|-----------|
| **model** | `generate model <name> [field:type ...]` | with-db only (`#[cfg(feature="with-db")]`). Creates `_entities/`, model, migration. | lib.rs:239 |
| **migration** | `generate migration <name> [field:type ...]` | with-db only. Standalone migration; name-based ops (Add/Remove/Create/Join…). | lib.rs:250, cli.rs:250 |
| **scaffold** | `generate scaffold <name> [fields] --api\|--html\|--htmx` | with-db only. Full CRUD: entity+migration+controller+views+tests. `ScaffoldKind` = Api/Html/Htmx (lib.rs:218). | lib.rs:261, cli.rs:268 |
| **controller** | `generate controller <name> [actions] --api\|--html\|--htmx` | Controller + routes + tests, no model. | lib.rs:274, cli.rs:307 |
| **task** | `generate task <name>` | One-off/CLI task stub + registration. | lib.rs:284 |
| **scheduler** | `generate scheduler` | Scheduler config scaffold. | lib.rs:288 |
| **worker** | `generate worker <name>` | Background worker stub + registration. | lib.rs:289 |
| **mailer** | `generate mailer <name>` | Mailer + embedded templates (subject.t/html.t/text.t). | lib.rs:293 |
| **data** | `generate data <name>` | Data loader scaffold (static data folder). | lib.rs:297 |
| **deployment** | `generate deployment --kind docker\|nginx` | `DeploymentKind` = Docker{copy_paths,is_client_side_rendering} / Nginx{host,port} (lib.rs:225). | lib.rs:301 |
| **override** | `generate override ...` | Copies built-in templates into the app so users take control of them (per-file / per-folder / all). | cli.rs:373 |

Default `ScaffoldKind`/controller kind resolution: `--htmx` → Htmx, `--html` → Html, else Api (cli.rs:419-425).

## Field-type mini-language (mappings.json — COMPLETE, ~50 base types)

Suffix convention: **(none)** = nullable `Option<T>`, **`!`** = required/non-null, **`^`** = unique (non-null). Verified against `mappings.json`.

- **uuid**: `uuid` `Option<Uuid>`, `uuid!` `Uuid`, `uuid^` = `uuid_uniq`.
- **string**: `string`/`string!`/`string^`. **text**: `text`/`text!`/`text^`.
- **Integers (1.0: `int` is now i64/BIGINT):**
  - `small_int`/`!`/`^` → i16. `int`/`!`/`^` → **i64 / big_integer** (⚠ was i32 pre-1.0). `big_int`/`!`/`^` → i64 (alias of int).
  - `small_unsigned`/`!`/`^` → i16. `big_unsigned`/`!`/`^` → i64. `unsigned`/`!`/`^` → **i64 / big_unsigned** (aliases big_unsigned).
- **Floats:** `float`/`!`/`^` → f32. `double`/`!`/`^` → f64.
- **Decimal:** `decimal`/`!`/`^` → Decimal. `decimal_len`/`!`/`^` → Decimal, **arity 2** (precision,scale).
- **bool**: `bool`/`bool!` (no unique).
- **Time:** `tstz`/`!` (DateTimeWithTimeZone), `date`/`!`/`^`, `date_time`/`!`/`^`.
- **JSON:** `json`/`!`, `jsonb`/`!`/`^`.
- **Binary:** `blob`/`!`/`^`; `binary_len`/`!`/`^` (arity 1); `var_binary`/`!`/`^` (arity 1).
- **money**: `money`/`!`/`^` → Decimal.
- **array**/`!`/`^` (arity 1): element ∈ {string,int,big_int,float,double,bool} → `Option<Vec<T>>`.

Special reference field: `<name>:references` (belongs-to FK) — handled in migration/model generators (see infer.rs). FK columns emitted as `BigInteger` to match i64 PKs (1.0).

## Inference conventions (infer.rs)
- **cruet** = pluralization ONLY (table names). **heck** = ALL casing (PascalCase entity, snake_case cols). Documented convention added in 0.18 work (comment-only). Empirical: mixing them corrupts casing (e.g. `i32`→`i_32`).

## Current doc coverage
- **MISSING**: no single generator reference page. Field-type mini-language is shown partially/inconsistently in `the-app/models.md` and `getting-started/guide.md`.
- **STALE**: any doc still showing `int` as 32-bit is wrong for 1.0 (now i64). Verify every `field:type` example against this table.
- **THIN**: `scaffold`/`controller` `--api/--html/--htmx` kinds, `deployment` (docker/nginx), `data`, and `override` are under- or un-documented as first-class generators.

## 1.0 priorities
1. A dedicated **Generators & CLI reference** page: every `generate <kind>` + the COMPLETE field-type table above.
2. Fix all `int`→i64 examples framework-wide (this is the highest-frequency stale fact).
3. Document `override` (template control) and `deployment` generators — currently near-invisible.
