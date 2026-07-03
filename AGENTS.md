# Building Loco apps — agent guide

This file teaches an AI agent how to build **Loco** (loco.rs) applications
correctly. Loco is an all-in-one, batteries-included Rust web framework (think
"Rails for Rust"): one binary and one set of conventions give you routing,
an ORM, background jobs, a scheduler, mailers, tasks, storage, caching, and
testing. Because it is batteries-included, the single most common failure mode
for LLMs is **reaching for external crates and hand-wiring infrastructure that
Loco already provides**. Prefer Loco's built-ins and generators.

This guide targets **Loco 0.17.x** (Sea-ORM 2.0, sqlx 0.9, edition 2021, MSRV
per the workspace `Cargo.toml`). For prose docs see https://loco.rs/docs, and
for a single-file reference see https://loco.rs/llms-full.txt.

## Golden rules

1. **Use the generators.** `cargo loco generate model|scaffold|controller|
   worker|task|scheduler|mailer|migration ...` writes correct, convention-
   following code. Generate, then edit — don't hand-write boilerplate.
2. **Everything hangs off `AppContext`.** Handlers, workers, tasks, and
   initializers receive `&AppContext` (`ctx`). `ctx.db` is the Sea-ORM
   connection; `ctx.config`, `ctx.mailer`, `ctx.storage`, `ctx.cache`,
   `ctx.queue_provider` are the other services. Do not create your own DB pool,
   HTTP server, or job queue.
3. **`use loco_rs::prelude::*;`** at the top of controllers/models/workers/tasks
   brings in the common types (`AppContext`, `Result`, `Routes`, `Json`,
   `State`, the Sea-ORM traits, etc.). If a common type is "missing", it is
   almost always in the prelude.
4. **`Result<T>` is `loco_rs::Result<T>`** and `Error` is `loco_rs::Error`
   (now `#[non_exhaustive]` — match with a `_ =>` arm). Use `?`; don't invent
   your own error enum for app code.
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
do this for you).

## Models & migrations (Sea-ORM 2.0)

- Generate: `cargo loco generate model posts title:string content:text
  user:references`. This writes a migration and regenerates the entity.
- Apply: `cargo loco db migrate`; regenerate entities: `cargo loco db entities`.
- **Primary and foreign keys are 64-bit (`i64` / BIGINT) in 0.17+.** Generated
  `id` columns and `references` are `i64`. Match key types when relating tables.
- Entities live in `src/models/_entities/` and are **generated** — put custom
  logic in `src/models/<name>.rs` (e.g. `ActiveModelBehavior`, finders).
- Query with Sea-ORM: `Entity::find_by_id(id).one(&ctx.db).await?`,
  `Entity::find().filter(Column::Field.eq(x)).all(&ctx.db).await?`. Create/update
  via `ActiveModel` + `.insert`/`.update`/`.save`.
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
  derive.

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
// enqueue at a priority (higher runs first):
DownloadWorker::perform_later_with_priority(&ctx, args, Some(42)).await?;
```

- Backends: Postgres, SQLite, or Redis (config `workers.mode` +
  `queue.kind`). Register workers in `app.rs` `Hooks::connect_workers`.
- **0.17+:** `perform_later` returns the job id (`Result<String>`); priority is
  supported on all backends (mailer jobs default to priority `100`). The Redis
  backend uses a Sorted Set — drain old Redis queues when upgrading from 0.16.

## Scheduler, mailers, tasks

- **Scheduler:** cron-like jobs in `config/*.yaml` under `scheduler:`; run with
  `cargo loco scheduler` or `cargo loco start --scheduler`. Jobs run shell
  commands or registered tasks.
- **Mailers:** generate with `cargo loco generate mailer`; send with
  `Mailer::mail(&ctx, &email)`. Templates live under `src/mailers/<name>/`.
  Configure SMTP (incl. implicit TLS via `mailer.smtp.tls: implicit`) in config.
- **Tasks:** implement the `Task` trait; run with `cargo loco task <name>`.
  Great for admin/data operations that need `AppContext`.

## Configuration

`config/development.yaml`, `production.yaml`, `test.yaml`. Selected by
`LOCO_ENV`. Access through `ctx.config`. Secrets come from the environment via
the `get_env` Tera helper *inside* the YAML, e.g.
`password: "{{ get_env(name='SMTP_PASSWORD') }}"`. Don't scatter `std::env::var`
calls through app code.

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

- **0.17+:** the request helpers take an `impl AsyncFnOnce` — call
  `request::<App>(...)` (drop the old `::<App, _, _>` turbofish).
- Use `request_with_create_db::<App>(...)` for DB-backed tests, seed with
  fixtures, and snapshot with `insta`. `#[serial]` DB tests that share state.

## Common LLM pitfalls (avoid these)

- ❌ Adding `axum`, `sqlx`, `tokio`, `lettre`, a job runner, etc. directly and
  wiring a server by hand. ✅ They're already integrated behind Loco — use
  `ctx` and the generators.
- ❌ Hand-writing entities in `_entities/`. ✅ Generate via migrations.
- ❌ `i32` primary keys / `Path<i32>`. ✅ `i64` in 0.17+.
- ❌ `request::<App, _, _>(...)`. ✅ `request::<App>(...)` in 0.17+.
- ❌ Building routes without registering them in `app.rs`. ✅ Return `Routes`
  from `routes()` and register in `Hooks::routes`.
- ❌ Custom error types for handlers. ✅ Return `loco_rs::Result<Response>` and
  use `?`.
- ❌ Reading env vars directly. ✅ YAML config + `ctx.config` + `get_env`.

## The CLI you will use most

```
cargo loco start [--server-and-worker | --worker | --scheduler | --all]
cargo loco generate model|scaffold|controller|worker|task|scheduler|mailer|migration
cargo loco db migrate|entities|reset|seed
cargo loco task <name>
cargo loco routes         # list all routes
cargo loco doctor         # check environment / versions
```

When unsure, run `cargo loco generate <thing> --help` and read the produced code
— it is the canonical, up-to-date pattern.
