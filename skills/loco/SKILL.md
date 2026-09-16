---
name: loco
description: Use when writing, reviewing, or debugging a Loco app (loco-rs, "Rails for Rust") — anything involving AppContext, controllers, Sea-ORM models and migrations, background workers, tasks, the scheduler, mailers, middleware, or `cargo loco`. Carries Loco's doctrine, its full public API index, and task recipes, so you can write idiomatic Loco without guessing at API names or reading framework source.
---

# Building Loco apps

Loco is **Rails for Rust**. When unsure how something should work, the answer is
almost always "the way Rails does it." Where Loco diverges it is because Rust
forced it, never because Loco disagreed with Rails.

## The three rules that prevent most mistakes

1. **Generate, then edit.** `cargo loco generate <thing>` writes the file *and*
   the wiring (`mod` declarations, route registration, worker registration).
   Rust has no autoloading, so hand-written wiring is where "the code exists but
   is never reached" bugs come from.
2. **Use the batteries.** Loco ships an ORM, queue, scheduler, mailer, task
   runner, storage, cache, and test harness. Adding a crate — or hand-writing
   infrastructure — for something Loco already does is the most common way
   agent-written Loco code goes wrong. See the P1 table in `doctrine.md`.
3. **Fat model, slim controller.** Finders and creation on `impl Model`, state
   transitions on `impl ActiveModel`. Handlers parse, call one or two model
   methods, and render.

## Everything hangs off `AppContext`

Handlers receive it as `State(ctx): State<AppContext>`. Eight fields, and they
are the answer to most "how do I get at X" questions:

| Field | Use |
|---|---|
| `ctx.db` | `DatabaseConnection` — all Sea-ORM calls |
| `ctx.config` | typed `config/<env>.yaml`; **the only** source of settings |
| `ctx.mailer` | mailer transport (mailers use it via `&ctx`) |
| `ctx.storage` | file storage abstraction |
| `ctx.cache` | cache abstraction |
| `ctx.queue_provider` | background queue (workers use it via `&ctx`) |
| `ctx.shared_store` | `Arc<SharedStore>`, typed DI slot; populated in `Hooks::after_context` |
| `ctx.environment` | current environment |

## Which file does what

```
src/
  app.rs                  Hooks impl: routes, workers, tasks, seeds, initializers
  controllers/            thin handlers + `routes()` returning `Routes`
  models/
    _entities/            GENERATED from schema — never hand-edit
    <name>.rs             your domain logic; re-exports from _entities
  views/                  response DTOs — entities never go on the wire directly
  workers/                BackgroundWorker impls
  mailers/                mailer structs + templates/
  tasks/                  Task impls (the `rake task` equivalent)
  initializers/           startup wiring
migration/src/            Sea-ORM migrations
config/<env>.yaml         all settings, incl. middleware and scheduler
tests/requests/           request tests via `loco_rs::testing::prelude`
```

## Read next — pick by what you are doing

Three files cover the situation you are in. Read the matching one first:

| Situation | File |
|---|---|
| **Setting up a new app**, or you just landed in one | **`starter-app.md`** — what `loco new` asks, what it produces, and which existing file is the working example of the thing you are about to write |
| **Making a change** to an app | **`workflow.md`** — the generate → migrate → edit → verify loop, the generator list, the column DSL, and what to edit after each generator |
| **The compiler is angry** | **`errors.md`** — symptom → cause. Check here *before* changing code; the common errors point away from their own cause |

Then, by subject:

| File | When |
|---|---|
| **`doctrine.md`** | **Read this before writing any Loco code.** What good looks like, the seven Rust-forced divergences from Rails, the task/worker/scheduler decision table, and the ranked smell list. |
| `api-index.md` | You need to know whether a `loco_rs` symbol exists or what it is called. Generated from rustdoc — it cannot drift. Check here *before* guessing an API name. |
| `sea-orm-index.md` | **Anything touching the database.** A Loco app is Loco *plus* Sea-ORM, and the ORM is API you write directly, not a detail the framework hides. Its query surface is almost entirely traits — `ColumnTrait`, `QueryFilter`, `QueryOrder`, `PaginatorTrait` — so the method you want is on a trait, not on the type it appears to belong to. |
| `recipes/model-and-migration.md` | Adding or changing a model, columns, relations, migrations |
| `recipes/endpoint.md` | Adding a controller, route, params, response |
| `recipes/background-job.md` | Work that must outlive the request |
| `recipes/task-and-schedule.md` | CLI tasks and anything recurring |
| `recipes/mailer.md` | Sending email |
| `recipes/cache.md` | Caching anything. **Read before using `ctx.cache`** — the default cache is a black hole that silently stores nothing |
| `recipes/config.md` | Settings, environments, secrets, and anything that differs between dev and production |
| `recipes/middleware.md` | Middleware composition, ordering, config |
| `recipes/auth.md` | JWT, API keys, protecting handlers |
| `recipes/testing.md` | Request tests, model tests, fixtures, snapshots |

## Do not guess API names

`api-index.md` and `sea-orm-index.md` are the complete public surfaces of
`loco_rs` and `sea-orm`, generated from rustdoc JSON and grouped by module.
Symbols that sound plausible but **do not exist**: `loco_rs::app::AppState`,
`loco_rs::jobs::Job`, `loco_rs::controller::Controller`,
`loco_rs::mailer::send_mail`, `loco_rs::db::Pool`. If you are about to write an
import or call you have not verified, check the index first.

The inverse mistake is more common: a method that **does** exist and appears not
to, because Sea-ORM's query surface is traits and the trait is not imported.
`.filter()` works from the prelude and `.order_by_desc()` on the next line does
not. See `errors.md` §1 before concluding a method is wrong.

## Escape hatch — reading framework source

If the index and recipes do not answer it, the framework source is already on
disk as a Cargo dependency:

```sh
cargo pkgid loco-rs                       # exact version in use
ls ~/.cargo/registry/src/*/loco-rs-*/src  # the source tree
```

Use this **last**, and grep a specific module — not the tree. It is ~36k lines,
and it answers "how is the framework built," which is a *different question*
from "how should my app be written." Framework internals use patterns
application code must not copy. For "what does good app code look like," read
`doctrine.md` and the recipes instead.

## The CLI

```sh
cargo loco start                          # run the server
cargo loco generate model posts title:string content:text
cargo loco generate scaffold posts title:string --api
cargo loco generate controller|worker|mailer|task|migration <name>
cargo loco db migrate|entities|reset|seed
cargo loco task <name>                    # run a task
cargo loco scheduler --list               # inspect scheduled jobs
cargo loco routes                         # every registered route
cargo loco doctor                         # diagnose environment/config
cargo loco version
```

## Before you call it done

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- No entity serialized straight to a response; no `tokio::spawn`; no
  `std::env::var`; no `.unwrap()` in a handler; no edits under `_entities/`.
