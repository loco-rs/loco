# Loco Audit — Area Division (12 areas)

Core = `src/` (31,578 LOC) + `loco-gen/` (2,393) + `xtask/`. Division by domain boundary.

- **A1 · Boot & Lifecycle** — `src/app.rs` (712), `src/boot.rs` (611),
  `src/environment.rs` (138), `src/initializers/` (70), `src/banner.rs` (104),
  `src/lib.rs` (52), `src/prelude.rs` (57). The Hooks trait + boot orchestration.
- **A2 · HTTP Core & Routing** — `src/controller/{routes.rs(585),app_routes.rs(458),
  format.rs(655),mod.rs(253),monitoring.rs(460),describe.rs(42),backtrace.rs(67)}`,
  `src/controller/extractor/` NON-auth parts (`validate.rs(753)` goes to A8; keep the
  rest). Routing, response formats, request extraction.
- **A3 · Middleware** — all of `src/controller/middleware/` (incl. `_archive/` — a
  known suspect: dead code in live tree). 20 files, ~2,000 LOC.
- **A4 · Views & Templating** — `src/controller/views/` (engine, embedded, tera_builtins)
  + `src/tera.rs` (8). ~700 LOC.
- **A5 · Background Workers & Queues** — all of `src/bgworker/` (5,646): `redis.rs`(1800),
  `sqlt.rs`(1466), `pg.rs`(1246), `mod.rs`(889), `sql.rs`(245). KNOWN SUSPECT: three
  near-parallel backends — quantify duplication precisely.
- **A6 · Storage** — all of `src/storage/` (3,573). KNOWN SUSPECT: `strategies/backup.rs`
  (1244) + `strategies/mirror.rs`(812) are huge for "strategies" — is it over-engineered?
- **A7 · Data Layer (ORM, Query DSL, Migrations, Schema)** — `src/db/` (1911),
  `src/model/` (1413), `src/schema.rs` (991). KNOWN SUSPECT: two schema files
  (`schema.rs` 991 + `db/schema.rs` 508) — clarify the split; query DSL `mod.rs` 968.
- **A8 · Auth, Validation & Security** — `src/auth/` (228),
  `src/controller/extractor/auth.rs` (860), `src/controller/extractor/validate.rs` (753),
  `src/validation.rs` (238), `src/hash.rs` (103).
- **A9 · Config, Errors & Observability** — `src/config/` (746), `src/env_vars.rs` (31),
  `src/errors.rs` (177), `src/logger.rs` (224), `src/doctor.rs` (388),
  `src/depcheck.rs` (251), `src/data.rs` (163), `src/cargo_config.rs` (171).
- **A10 · CLI & Code Generation** — `src/cli.rs` (1431) + `loco-gen/` crate (2,393).
  The generator template engine + field-type mini-language + `cargo loco` command surface.
- **A11 · Services: Mailer, Cache, Scheduler, Tasks** — `src/mailer/` (562),
  `src/cache/` (1243), `src/scheduler.rs` (628), `src/task.rs` (279).
- **A12 · Testing Infrastructure** — `src/testing/` (1314), `src/tests_cfg/` (590).
  The test harness Loco ships to its users (request/model test helpers, fixtures, redaction).
