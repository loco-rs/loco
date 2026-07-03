# Loco Data Layer / ORM Inventory (v1.0.0 doc overhaul)

Scope: `src/model/`, `src/db/`, `src/schema.rs`, `src/validation.rs`, `src/config/database.rs`, `src/initializers/{multi_db,extra_db}.rs`, `loco-gen`.
Verified against code on branch `release/0.17.0`. Everything below is grounded in `file:line`. Doc targets rated MISSING / THIN / STALE / ACCURATE.

Feature flag: the entire data layer is gated behind **`with-db`** (`Cargo.toml:44-49` → pulls `sea-orm`, `sea-orm-migration`, `sqlx`, `loco-gen/with-db`). `prelude.rs:29` and `prelude.rs:52` gate `query`, `ModelError`, `ModelResult`, `Authenticable` on `with-db`. `validation.rs:41` gates the `DbErr` conversion on `with-db`.

1.0 dependency reality (verify carefully for the migration guide):
- `sea-orm = "2.0.0-rc"` (`Cargo.toml:73-78`); comment at `Cargo.toml:7-8` notes MSRV raised for **Sea-ORM 2.0 + sqlx 0.9**, current tree `2.0.0-rc.41 / sqlx 0.9` needs rustc 1.94, 2.0 stable target 1.85.
- `serde_yaml` is actually **`serde_yaml_ng` 0.10** aliased to the `serde_yaml` name (`Cargo.toml` dep note). Used by seed loader.

---

## 1. ModelError / ModelResult — error type for model hooks
Purpose: normalized error enum returned by model/authn logic; wraps SeaORM + validation.
- `enum ModelError` — `src/model/mod.rs:13-35`. Variants: `EntityAlreadyExists`, `EntityNotFound`, `Validation(ModelValidationErrors)` (`#[from]`), `Jwt(...)` (only under `auth_jwt`, `mod.rs:23-25`), `DbErr(sea_orm::DbErr)` (`#[from]`), `Any(Box<dyn Error+Send+Sync>)` (`#[from]`), `Message(String)`.
- `type ModelResult<T, E = ModelError>` — `mod.rs:38`.
- Constructors: `ModelError::wrap` `mod.rs:42`, `to_msg` `mod.rs:47`, `msg` `mod.rs:52`.
- `trait Authenticable` — `mod.rs:56-60`: `find_by_api_key`, `find_by_claims_key` (used by auth extractors, implemented on the user model).
- Exported via `prelude`: `ModelError`, `ModelResult`, `Authenticable` (`prelude.rs:30`).
- Doc coverage (models.md): **MISSING**. models.md never documents `ModelError`/`ModelResult`/`Authenticable`; examples return raw `DbErr` (models.md:107). No enumeration of the error surface users hit.

## 2. Validation — model validation bridge
Purpose: validate ActiveModels before save, adapt the `validator` crate, and smuggle validation errors through `DbErr` for the central error handler.
- `trait Validatable` — `validation.rs:155-166`: `validator() -> Box<dyn Validate>` + provided `validate()`. This is the trait users implement on `ActiveModel`.
- `trait ValidatorTrait` — `validation.rs:137-144` with blanket impl for any `validator::Validate` (`validation.rs:147-151`) → gives `.validate()` returning `ModelValidationErrors`.
- `struct ModelValidationErrors` — `validation.rs:74-78` (`BTreeMap<String, Vec<ValidationError>>`); `struct ValidationError` `validation.rs:66-72`; `struct ModelValidationMessage` `validation.rs:52-55`.
- `From<ValidationErrors> for ModelValidationErrors` `validation.rs:80`; `From<ModelValidationErrors> for DbErr` `validation.rs:103` (with-db); `into_db_error()` `validation.rs:111` encodes errors as JSON inside `DbErr::Custom`.
- Prelude exports: `validation`, `Validatable`, `ValidatorTrait`, and `validator::Validate` (`prelude.rs:48,51`).
- Doc coverage (models.md "Validation" §612-641): **ACCURATE but THIN**. Shows `Validatable`/`validator()` correctly. Does NOT document: `ValidatorTrait` blanket impl (custom validation without the `validator` crate — see `validation.rs` test `CustomValidator`), the `ModelValidationErrors` shape, or how errors surface as HTTP responses via the `DbErr::Custom` JSON hack (`validation.rs:57-65`). The `before_save`+`validate()` wiring is only in the `validation.rs` module doc (`validation.rs:26-38`), not the site docs.

## 3. Query DSL — `ConditionBuilder` fluent filter builder
Purpose: build SeaORM `Condition`s ergonomically; used with `paginate`.
- Module: `src/model/query/dsl/mod.rs`; re-exported `src/model/query/mod.rs:4`. Reached by users as `query::condition()...` (prelude `query` = `crate::model::query`, `prelude.rs:30,54`).
- Entry fns: `condition()` `dsl/mod.rs:38`, `with(Condition)` `dsl/mod.rs:45`.
- Free-fn shortcuts (each = `condition().<op>`): `eq` `:51`, `not_equal` `:57`, `gt` `:63`, `gt_equal` `:69`, `lt` `:75`, `lt_equal` `:81`, `between` `:87`, `not_between` `:93`, `like` `:99`, `not_like` `:105`, `starts_with` `:111`, `ends_with` `:117`, `contains` `:123`, `is_null` `:130`, `is_not_null` `:137`, `is_in` `:144`, `is_not_in` `:154`, `date_range` `:163`.
- `impl ConditionBuilder` methods: `eq` `:235`, `ne` `:260`, `gt` `:285`, `gte` `:311`, `lt` `:337`, `lte` `:363`, `between` `:389`, `not_between` `:415`, `like` `:441`, `not_like` `:467`, `starts_with` `:493`, `ends_with` `:519`, `contains` `:545`, `is_null` `:572`, `is_not_null` `:599`, `is_in` `:626`, `is_not_in` `:657`, `date_range` `:696`, `build() -> Condition` `:701`. `From<ConditionBuilder> for Condition` `:167`.
- `enum SortDirection { Desc, Asc }` with serde rename + `order() -> Order` — `dsl/mod.rs:17-35`.
- `DateRangeBuilder<T>` — `dsl/date_range.rs:7-66`: `new` `:15`, `dates(from,to)` `:25`, `from` `:35`, `to` `:45`, `build() -> ConditionBuilder` `:54`. Note asymmetry: open-ended `from` uses `>` (gt), `to` uses `<` (lt), both-ends uses BETWEEN (`date_range.rs:55-63`) — worth documenting the strict-vs-inclusive boundary behavior.
- Doc coverage: **MISSING from site docs.** models.md / data.md never mention the query DSL, `condition()`, `date_range`, or `SortDirection`. Only rustdoc examples exist (in `dsl/mod.rs`). This is a substantial undocumented public surface (~18 operators).

## 4. Pagination — `paginate` / `fetch_page`
Purpose: page over entities/selectors, return data + meta.
- `struct PaginationQuery { page_size: u64, page: u64 }` — `paginate/mod.rs:31-45`. Defaults: `page_size=25` (`:5`), `page=1` (`:10`). `#[serde(flatten)]`-friendly; custom `deserialize_pagination_filter` (`:69`) works around a `serde_urlencoded` bug (parses strings→u64). `PaginationQuery::page(n)` ctor `:49`. `Default` `:58`.
- `struct PageResponse<T> { page: Vec<T>, meta: PagerMeta }` — `paginate/mod.rs:80-83` (`PagerMeta` from `controller::views::pagination`).
- `async fn paginate<E>(db, Select<E>, Option<Condition>, &PaginationQuery)` — `paginate/mod.rs:146`. Uses `num_items_and_pages()`; 1-based page in, `saturating_sub(1)` internally (`:156`).
- `async fn fetch_page<C,S>(db, selector, &PaginationQuery)` — `paginate/mod.rs:204` (generic over any `PaginatorTrait`).
- Reached as `query::paginate` / `query::fetch_page` / `query::PaginationQuery`.
- Doc coverage: **MISSING from models.md/data.md.** Pagination is only covered by rustdoc + (likely) the controllers/views docs. `PaginationQuery`, `PageResponse`, `paginate`, `fetch_page` are unlisted in the data-layer docs.

## 5. Migration schema DSL (`src/schema.rs`, exported as `loco_rs::schema::*`)
Purpose: Rails-like migration authoring helpers over SeaORM `sea_query`.
- `enum ColType` — `schema.rs:161-284`: ~140 variants covering Pk (`PkAuto`, `PkUuid`), char/string/text (+Null/Uniq/WithDefault/Len), integer families (Integer/Small/Big/Unsigned/BigUnsigned...), decimal/float/double/money, bool, date/time/datetime/timestamptz, interval, binary/varbinary/blob, json/jsonb, uuid, varbit, **array** (`Array`/`ArrayNull`/`ArrayUniq` + `ArrayColType` helper enum `:286-293`, ctors `array`/`array_uniq`/`array_null` `:298-311`), and **enum** (`Enum`/`EnumNull`/`EnumWithDefault`/`EnumNullWithDefault` `:280-283`). `ColType::to_def()` maps to column defs `:328-470`.
- **CRITICAL 1.0 change** — `ColType::PkAuto => big_pk_auto(name)` (`schema.rs:330-332`): default auto PK is now **64-bit (i64/BIGINT)**, not i32. Comment cites Sea-ORM 2.0 mapping SQLite ints→i64 and Rails-5.1-style bigint default. FK columns generated for `references` are `BigInteger`/`BigIntegerNull` to match (`schema.rs:677-683`, `add_reference` `:782`).
- Table ops (all `async`, `SchemaManager`-based): `create_table` `:490`, `create_join_table` `:512` (composite PK), `create_table_without_timestamps` `:537`, `create_join_table_without_timestamps` `:559`, `add_column` `:721`, `remove_column` `:745`, `add_reference` `:764`, `remove_reference` `:849`, `drop_table` `:902`, `add_enum_values` `:916`, `drop_enum_type` `:962`.
- Column-def helpers: `alter` `:19`, `table_auto_tz` `:24`, `timestamps_tz` `:34`, `timestamptz`/`timestamptz_null` `:42/:53`, `enum_type`/`enum_type_null`/`enum_type_with_default`/`enum_type_null_with_default` `:64/:75/:93/:112`. Re-exports `sea_orm_migration::schema::*` (`schema.rs:9`).
- Behavior worth documenting: timestamps auto-added unless `_without_timestamps` (`create_table_impl` `add_timestamps` `:574,629`); table names auto-pluralized+snake_cased (`normalize_table` `:704`, uses `cruet`); FK naming `fk-{from}-{ref}-to-{to}` (`:687`); nullable ref via `?` suffix → `ON DELETE SET NULL` else `CASCADE` (`:664-697`); SQLite cannot add/drop FK on existing table (no-op, `:808-838`, `:879`); Postgres enums auto-created if missing, SQLite/MySQL no-op (`:578-626`).
- Doc coverage (models.md "Authoring migrations" §412-610): **THIN / PARTIALLY STALE.** Documents `create_table`, `create_join_table`, `add_column`, `remove_column`, `drop_table`, and enum types (Enum variants list §597-602 ACCURATE). BUT: (a) the full `ColType` surface (~140 variants: money, varbit, interval, array helpers, `*WithDefault`, `*Len`) is undocumented; (b) `add_reference`/`remove_reference`, `add_enum_values`, `create_*_without_timestamps` fns are not shown as DSL fns; (c) **does not state the i64 PK default** — a headline 1.0 change; (d) internal doc-comment at `schema.rs:533` still says CLI flag `--without-timestamps` while the real flag is `--without-tz` (see §9) — a stale example.

## 6. DB connection / lifecycle (`src/db/connect.rs`)
Purpose: build SeaORM connections from config; DB creation; access checks.
- `async fn connect(&config::Database) -> Result<DbConn, DbErr>` — `connect.rs:90`. Applies pool opts, `sqlx_logging`, optional `acquire_timeout`; on connect runs `run_on_start` — SQLite gets a **default PRAGMA block** (`connect.rs:105-121`: foreign_keys ON, WAL, synchronous NORMAL, mmap 128MiB, journal_size_limit 64MiB, cache_size 2000, busy_timeout 5000) unless overridden; Postgres/MySQL run `run_on_start` only if set.
- `struct MultiDb { db: HashMap<String,DatabaseConnection> }` — `connect.rs:19-52`: `MultiDb::new(HashMap<String, config::Database>)` `:30`, `get(name)` `:47`.
- `async fn verify_access(&db)` — `connect.rs:60` (Postgres: checks user owns tables).
- `fn extract_db_name(conn_str)` — `connect.rs:149` (regex `EXTRACT_DB_NAME`).
- `async fn create(db_uri)` — `connect.rs:161`: **Postgres-only** DB creation; uses env `LOCO_POSTGRES_DB_OPTIONS` (default `ENCODING='UTF8'`) via `create_postgres_database` `:182`.
- Doc coverage: **THIN.** models.md documents the SQLite default PRAGMAs indirectly only via the config field doc (`config/database.rs:66-83`). `run_on_start`, `verify_access`, `create`, `extract_db_name`, `MultiDb::new/get` API not in site docs (MultiDb usage shown at models.md:944-955 but not the type's API).

## 7. Migrations runtime (`src/db/migrate.rs`)
Purpose: converge/run migrations at boot and via CLI.
- `async fn converge<H,M>(ctx, &config::Database)` — `migrate.rs:17`: honors `dangerously_recreate` → `reset` (`:21`), `auto_migrate` → `migrate` (`:27`), `dangerously_truncate` → `H::truncate` (`:32`).
- `migrate<M>` `:44` (`M::up`), `down<M>(db, steps)` `:53` (`M::down(Some(steps))`), `status<M>` `:65`, `reset<M>` `:75` (`M::fresh` then migrate).
- Doc coverage (models.md §288-391): **ACCURATE** for CLI workflow (`db migrate`, `db down`, `db down N`, `db entities`). The `converge` boot flow tie to config flags is documented at models.md:735-737 (ACCURATE). Programmatic `migrate/down/status/reset` fns are not listed but low priority.

## 8. Seeding + schema dump/introspection (`src/db/seed.rs`, `src/db/schema.rs`)
Purpose: load fixtures, reset sequences, dump tables/schema.
- `async fn seed<A: ActiveModelTrait>(db, path)` — `seed.rs:24`: reads YAML → `Vec<Value>` → `A::from_json` → `insert_many`, then `reset_autoincrement` (`seed.rs:52`).
- `async fn reset_autoincrement(backend, table, db)` — `seed.rs:163` (public); helpers `has_id_column` `:73`, `is_auto_increment` `:123` (Postgres serial / SQLite AUTOINCREMENT detection). `run_app_seed<H>` `:220`.
- `db/schema.rs`: `truncate_table<T>` `:23`, `get_tables` `:38` (skips `IGNORED_TABLES`), `dump_tables(db, to, only_tables)` `:147` (YAML per table, booleans dumped as true/false via `get_boolean_columns` `:88`, JSON/UUID/datetime handling), `dump_schema(ctx, fname)` `:272` (JSON schema per backend).
- `IGNORED_TABLES` = `seaql_migrations`, `pg_loco_queue`, `sqlt_loco_queue`, `sqlt_loco_queue_lock` — `db/mod.rs:18-23`.
- Doc coverage (models.md "Seeding" §739-851 + CLI §806-832): **ACCURATE.** `db::seed::<ActiveModel>` wiring in `Hooks::seed` shown correctly; `db seed --reset/--dump/--dump-tables/--from` documented. `reset_autoincrement`, `dump_schema`, `truncate_table`, `get_tables` API not called out but covered functionally.

## 9. Entities generation (`src/db/entities.rs`)
Purpose: `cargo loco db entities` → run `sea-orm-cli generate entity` + post-process.
- `async fn entities<M>(ctx)` — `entities.rs:101`: checks seaorm CLI + DB, builds flags. Defaults (`EntityCmd::new` `:16-33`): `--database-url <uri>`, `--ignore-tables <IGNORED_TABLES,...>`, `--output-dir src/models/_entities`, `--with-serde both`, `--with-copy-enums`.
- Cargo.toml overrides via `[package.metadata.db.entity]` → `merge_with_config` `:35` (`--output-dir`/`--database-url` cannot be overridden `:43-49`; `ignore-tables` appends `:50-57`).
- `fix_entities()` `:134`: strips `impl ActiveModelBehavior for ActiveModel {}` from generated files and scaffolds an extension module in `src/models/<name>.rs` with a `before_save` that auto-touches `updated_at` when the entity has that column (`entities.rs:188-214`).
- Doc coverage (models.md §1079-1092 "Customizing Entity Generation"): **ACCURATE** for the Cargo.toml metadata knobs. The default flags and the `updated_at` auto-touch `before_save` generation are undocumented (THIN) — users won't know `updated_at` is auto-managed.

## 10. Generators (loco-gen) — model / migration / scaffold
- Type mapping source of truth: `loco-gen/src/mappings.json` (`field_types`). Model/migration/scaffold generators + `mappings.json` all live under `loco-gen`; `with-db` feature also toggles `loco-gen/with-db`.
- CLI flags confirmed in `src/cli.rs`: model `--without-tz` (`cli.rs:201-203`), migration/join-table `--without-tz` (`cli.rs:236-242`). Field suffixes `!` = required/NOT NULL, `^` = unique; `references` / `references:<col>` / `references?` (nullable) per models.md:240-267 (ACCURATE vs `schema.rs` ref logic).
- **STALE data-type table in models.md §170-238** — the doc's hardcoded mapping list disagrees with `mappings.json`:
  - `("int", "integer_null")`, `("int!", "integer")`, `("int^", "integer_uniq")` — **WRONG.** `mappings.json:111-128` maps `int`→`big_integer_null` (`Option<i64>`), `int!`→`big_integer` (`i64`), `int^`→`big_integer_uniq`. This is the i64 change; the doc still shows 32-bit.
  - `("unsigned!", "unsigned")` / `("unsigned^", "unsigned_uniq")` — **WRONG.** `mappings.json:349-364` maps `unsigned`→`big_unsigned*` (`i64`).
  - `("big_unsigned^", "big_unsigned")` and `("big_unsigned!", "big_unsigned_uniq")` — **SWAPPED** vs `mappings.json:81-92` (`!`→non-uniq, `^`→uniq).
  - Array rows garbled with leading spaces `(" array", "array")` (models.md:235-237) vs real `array`/`array!`/`array^` with per-inner-type Rust mapping (`mappings.json:408-449`).
  - Rust-side types: `mappings.json` emits `Option<i64>`/`i64` for int/unsigned — reinforces the 64-bit story the prose omits.

## 11. Multi-DB initializers (`src/initializers/`)
- `ExtraDbInitializer` — `extra_db.rs:10-36`: reads `initializers.extra_db` config, `db::connect`, layers a single extra `DatabaseConnection` as axum `Extension`. **Has leftover `println!("1"/"2"/"3")` debug lines** (`extra_db.rs:19,25,30`) — a real code smell to flag for 1.0 cleanup.
- `MultiDbInitializer` — `multi_db.rs:10-32`: reads `initializers.multi_db` map, `db::MultiDb::new`, layers `MultiDb` extension.
- Doc coverage (models.md §853-956): **ACCURATE.** Both YAML shapes and `Extension<DatabaseConnection>` / `Extension<MultiDb>` controller usage documented. (Docs don't mention the stray `println!`s — not a doc issue but flag for eng.)

---

## Config knobs — `config::Database` (`src/config/database.rs:24-84`, YAML key `database:`)
`uri` (String), `enable_logging` (bool → sqlx logging), `min_connections` (u32), `max_connections` (u32), `connect_timeout` (u64 ms), `idle_timeout` (u64 ms), `acquire_timeout` (Option<u64> ms), `auto_migrate` (bool, default false), `dangerously_truncate` (bool), `dangerously_recreate` (bool), `run_on_start` (Option<String>).
- models.md config snippet (§703-733): **STALE / INCOMPLETE** — omits **`acquire_timeout`** and **`run_on_start`** (both real, `database.rs:46,83`). The default `min/max` helper fns (`db_min_conn=1`, `db_max_conn=20`, `db_connect_timeout=500`, `db_idle_timeout=500`, `database.rs:86-100`) are not surfaced.

## Env vars (data layer)
- `LOCO_POSTGRES_DB_OPTIONS` — `env_vars.rs:8`, used by `connect::create_postgres_database` (default `ENCODING='UTF8'`). **Undocumented in models.md.**
- `LOCO_DATA` (`LOCO_DATA_FOLDER_ENV`, `env_vars.rs:20`) — belongs to the static data loader, documented in data.md:49.

## Note on `infrastructure/data.md`
This file documents the **static `data/` loader** (`data::stocks::get()/read()`, `LOCO_DATA`) — it is NOT about the DB/ORM at all. The task brief pairs it with the data layer, but there is zero ORM overlap. Data-layer docs live essentially only in `the-app/models.md`.
