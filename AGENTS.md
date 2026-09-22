# Building Loco apps — agent guide

This file teaches an AI agent how to build **Loco** (loco.rs) applications
correctly. Loco is an all-in-one, batteries-included Rust web framework (think
"Rails for Rust"): one binary and one set of conventions give you routing,
an ORM, background jobs, a scheduler, mailers, tasks, storage, caching, and
testing. Because it is batteries-included, the single most common failure mode
for LLMs is **reaching for external crates and hand-wiring infrastructure that
Loco already provides**. Prefer Loco's built-ins and generators.

This guide targets **Loco 1.0** (Sea-ORM 2.0, sqlx 0.9, edition 2024 for the
framework itself — generated apps are still edition 2021). For prose docs see
https://loco.rs/docs, and for a single-file reference see
https://loco.rs/llms-full.txt.

## Golden rules

1. **Use the generators.** `cargo loco generate model|scaffold|controller|
   worker|task|scheduler|mailer|migration|deployment|override ...` writes
   correct, convention-following code. Generate, then edit — don't hand-write
   boilerplate. `scaffold`/`controller` require exactly one of `--api`/
   `--html`/`--htmx` (no default; omitting all is a hard error).
2. **Everything hangs off `AppContext`.** Handlers, workers, tasks, and
   initializers receive `&AppContext` (`ctx`), with 8 fields: `db` (`with-db`
   only), `config`, `mailer`, `storage`, `cache`, `queue_provider`,
   `shared_store` (a type-keyed DI container), `environment`. Do not create
   your own DB pool, HTTP server, or job queue.
3. **`use loco_rs::prelude::*;`** at the top of controllers/models/workers/tasks
   brings in the common types (`AppContext`, `Result`, `Routes`, `Json`,
   `State`, the Sea-ORM traits, JWT auth extractors under `auth`, etc.).
   If a common type is "missing", it is almost always in the prelude.
4. **`Result<T>` is `loco_rs::Result<T>`** and `Error` is `loco_rs::Error`
   (`#[non_exhaustive]` — match with a `_ =>` arm; `EnvVar`/`Hash`/`SemVer`/
   `TaskJoinError` were removed). Use `?`; don't invent your own error enum
   for app code.
5. **Config is YAML per-environment** in `config/*.yaml`, read through
   `ctx.config`. Don't read env vars ad hoc; use the config + `get_env` Tera
   helper inside the YAML.

## Project layout

```
src/
  app.rs                 # Hooks impl: registers routes, workers, tasks, etc.
  lib.rs / main.rs / bin/
  controllers/           # HTTP handlers, grouped into Routes
  models/
    _entities/           # Sea-ORM entities (generated; don't hand-edit)
    *.rs                 # your model logic (ActiveModel hooks, finders)
  views/                 # response shaping (JSON/HTML)
  workers/               # background jobs
  tasks/                 # one-off / CLI tasks
  mailers/               # email
  initializers/          # startup hooks
migration/               # Sea-ORM migrations (separate crate)
config/                  # development.yaml, production.yaml, test.yaml
tests/                   # request/model/task tests
assets/ frontend/        # static assets / SPA (optional)
```

The `App` type implements the `Hooks` trait in `src/app.rs`. That is where you
**register** routes, workers, tasks, and initializers — a newly generated
controller/worker/task is not active until it is wired in there (the generators
do this for you). `Hooks::boot`'s second parameter is `environment:
&Environment` (the enum), not `&str` — copy from generated code, not memory.

## Models & migrations (Sea-ORM 2.0)

- Generate: `cargo loco generate model posts title:string! content:text
  user:references`. This writes a migration and regenerates the entity.
- Apply: `cargo loco db migrate`; regenerate entities: `cargo loco db entities`.
- **Primary and foreign keys are 64-bit (`i64` / BIGINT) in 1.0.** The `int`
  field type is also `i64`/BIGINT now (it was `i32` pre-1.0) — match key
  types when relating tables. `small_int` still maps to `i16` if you need it.
- Entities live in `src/models/_entities/` and are **generated** — put custom
  logic in `src/models/<name>.rs` (e.g. `ActiveModelBehavior`, finders).
- Query with Sea-ORM: `Entity::find_by_id(id).one(&ctx.db).await?`,
  `Entity::find().filter(Column::Field.eq(x)).all(&ctx.db).await?`. Create/update
  via `ActiveModel` + `.insert`/`.update`/`.save`. For ad-hoc filters, prefer
  Loco's `query::condition()...build()` DSL (`eq`, `like`, `contains`,
  `is_in`, `date_range`, ~18 ops) over hand-rolling `Condition`s.
- Pagination: `query::paginate(&ctx.db, Entity::find(), Some(condition),
  &pagination_query).await?` → `PageResponse { page, meta: PagerMeta { page,
  page_size, total_pages, total_items } }`.
- Sea-ORM 2.0 note: raw-`Statement` execution methods carry a `_raw` suffix
  (`execute_raw`, `query_one_raw`, `query_all_raw`); most apps never touch these.

## Controllers & routing

```rust
use loco_rs::prelude::*;

pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Entity::find().all(&ctx.db).await?)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/posts/")
        .add("/", get(list))
        .add("{id}", get(get_one))
        .add("/", post(add))
}
```

- Handlers are `async fn(State(ctx): State<AppContext>, ...) -> Result<Response>`.
  Extract a body with `Json(params): Json<Params>`, a path with `Path(id):
  Path<i64>`, query with `Query(...)`.
- Build a `Routes` group with `.prefix(...)` + `.add(path, method(handler))` and
  return it from `routes()`; register it in `app.rs` `Hooks::routes`.
- Shape responses with `format::json(...)`, `format::html(...)`, or the view
  layer. Validate request bodies with the `JsonValidate` extractor + `validator`
  derive. Errors map to HTTP: `NotFound`→404, `Unauthorized`→401,
  `BadRequest`/`Validation`→400, everything else (DB, IO, etc.)→500.

## Authentication (`auth`, default feature)

- Feature is named **`auth`**. Default signing algorithm is
  **HS512**; `auth.jwt.secret` **must be valid base64** — plain strings fail
  at token-generate/validate time, not config-load time.
- Extractors: `auth::JWT` (claims only, no DB needed), `auth::JWTWithUser<T>`
  (claims + loaded user, needs `with-db`), `auth::ApiToken<T>` (bearer API
  key → user, needs `with-db`; always reads the `Authorization: Bearer`
  header regardless of `auth.jwt.location`).
- `JWTWithUser`/`ApiToken` require your user model to implement
  `loco_rs::model::Authenticable` (`find_by_api_key`, `find_by_claims_key`).
- Password hashing: `loco_rs::hash::{hash_password, verify_password,
  random_string}` (Argon2id, always compiled).

## Background workers (with priority)

```rust
use loco_rs::prelude::*;

pub struct DownloadWorker;

#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self { Self }
    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> { Ok(()) }
}

// enqueue (returns the job id):
let job_id = DownloadWorker::perform_later(&ctx, args).await?;
// enqueue at a priority (higher runs first), on ANY backend:
DownloadWorker::perform_later_with_priority(&ctx, args, Some(100)).await?;
// enqueue many jobs in ONE round trip (atomic on every backend); returns
// one id per job, in input order:
let job_ids = DownloadWorker::perform_all_later(&ctx, args_list).await?;
// same, with a priority per job (None = default):
DownloadWorker::perform_all_later_with_priority(&ctx, vec![(a, Some(100)), (b, None)]).await?;
```

- Backends: Postgres and SQLite ship by default (feature `worker`); Redis needs
  the `worker_redis` feature. The backend is chosen at runtime (config
  `workers.mode` + `queue.kind`). Register workers in `app.rs`
  `Hooks::connect_workers`.
- `perform_later`/`perform_later_with_priority` return the job id
  (`Result<String>`). Priority (full `i32` range) works on all three
  backends. Redis fully supports job admin now (cancel/clear/requeue/dump/
  import) — it is not Postgres/SQLite-only.
- ❌ Calling `perform_later` in a loop to fan out N jobs. ✅
  `perform_all_later(&ctx, Vec<Args>)` — one round trip, one transaction.
- Manage from the CLI: `cargo loco jobs cancel|tidy|purge|dump|import|requeue`.

## Scheduler, mailers, tasks

- **Scheduler:** cron-like jobs in `config/*.yaml` under `scheduler:`; run with
  `cargo loco scheduler` or `cargo loco start --scheduler`. Jobs run shell
  commands or registered tasks.
- **Mailers:** generate with `cargo loco generate mailer`; send with
  `Mailer::mail`/`mail_template`. Templates live under `src/mailers/<name>/`.
  Configure SMTP TLS explicitly — `mailer.smtp.tls: starttls|implicit|none`
  **overrides** the legacy `secure` bool; implicit TLS / port 465 (SMTPS)
  needs `tls: implicit`, since `secure: true` alone only ever means STARTTLS.
- **Tasks:** implement the `Task` trait; run with `cargo loco task <name>`.
  Great for admin/data operations that need `AppContext`.

## Configuration

`config/development.yaml`, `production.yaml`, `test.yaml`. Selected by
`LOCO_ENV` → `RAILS_ENV` → `NODE_ENV` → `development`. `{env}.local.yaml`
overrides `{env}.yaml` when both exist. Access through `ctx.config`. Secrets
come from the environment via the `get_env` Tera helper *inside* the YAML,
e.g. `password: "{{ get_env(name='SMTP_PASSWORD') }}"`. Don't scatter
`std::env::var` calls through app code.

## Testing

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_list() {
    request::<App>(|request, _ctx| async move {   // NOTE: ::<App>, not ::<App, _, _>
        let res = request.get("/api/posts/").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}
```

- Requires the `testing` feature (off by default in a plain lib dep, on for
  the generated app's dev-dependencies).
- Request helpers take a callback `|request, ctx| async move { ... }` — call
  `request::<App, _, _>(...)`. Boot helper `boot_test::<H>()` is
  **single-generic** (not `boot_test::<App, Migrator>()`).
- Use `request_with_create_db::<App, _, _>(...)` for DB-backed tests (fresh DB,
  auto-cleaned), seed with fixtures, and snapshot with `insta` (use
  `testing::redaction::cleanup_user_model()`/`cleanup_email()` filters).
  `#[serial]` DB tests that share state.

## Common LLM pitfalls (avoid these)

- ❌ Adding `axum`, `sqlx`, `tokio`, `lettre`, a job runner, etc. directly and
  wiring a server by hand. ✅ They're already integrated behind Loco — use
  `ctx` and the generators.
- ❌ Hand-writing entities in `_entities/`. ✅ Generate via migrations.
- ❌ `i32` primary keys / `Path<i32>`, or assuming `int` fields are 32-bit.
  ✅ `i64` everywhere in 1.0 (keys, FKs, and the `int` field type).
- ❌ `boot_test::<App, Migrator>()` (the old two-generic boot helper). ✅
  `boot_test::<H>()` in 1.0.
- ❌ Building routes without registering them in `app.rs`. ✅ Return `Routes`
  from `routes()` and register in `Hooks::routes`.
- ❌ Custom error types for handlers. ✅ Return `loco_rs::Result<Response>` and
  use `?`; match `Error` with a `_ =>` arm (it's `#[non_exhaustive]`).
- ❌ Reading env vars directly. ✅ YAML config + `ctx.config` + `get_env`.
- ❌ Assuming `secure: true` covers implicit TLS. ✅ Use `tls: implicit` for
  port 465.
- ❌ `scaffold`/`controller` generation without a kind flag. ✅ pass one of
  `--api`/`--html`/`--htmx` — there's no default.

## The CLI you will use most

```
cargo loco start [--server-and-worker | --worker | --scheduler | --all]
cargo loco generate model|scaffold|controller|worker|task|scheduler|mailer|
                 migration|deployment|override [--api|--html|--htmx]
cargo loco db migrate|entities|reset|seed
cargo loco task <name>
cargo loco jobs cancel|tidy|purge|dump|import|requeue
cargo loco routes         # list all routes
cargo loco doctor         # check environment / versions
```

`loco new` (the separate app-generator binary, `cargo install loco`) flags:
`--name --db <sqlite|postgres|none> --bg <async|queue|blocking> --assets
<serverside|clientside|none> --os <linux|windows|macos> --allow-in-git-repo`.
There is **no `--template`/`--verbose`** flag — template choice is
interactive-only.

When unsure, run `cargo loco generate <thing> --help` and read the produced code
— it is the canonical, up-to-date pattern.
