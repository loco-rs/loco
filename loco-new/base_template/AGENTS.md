# Agent guide for this Loco app

This is a **Loco** (loco.rs) application — an all-in-one, batteries-included Rust
web framework. Routing, the database (Sea-ORM), background jobs, a scheduler,
mailers, tasks, storage, caching, and testing are already integrated. **Prefer
Loco's built-ins and generators over adding external crates or wiring
infrastructure by hand.**

## Where things live

```
src/app.rs            # impl Hooks for App — registers routes/workers/tasks (the wiring hub)
src/controllers/      # HTTP handlers grouped into Routes
src/models/_entities/ # GENERATED Sea-ORM entities — do not hand-edit
src/models/*.rs       # your model logic
src/workers/          # background jobs
src/tasks/            # CLI/admin tasks
src/mailers/          # email
migration/            # Sea-ORM migrations
config/*.yaml         # per-environment config (LOCO_ENV)
tests/                # request/model/task tests
```

## How to work in this app

- **Add features with generators**, then edit:
  `cargo loco generate model|scaffold|controller|worker|task|mailer|migration ...`.
  The generators also wire new code into `src/app.rs`.
- **Everything uses `AppContext` (`ctx`)**: `ctx.db`, `ctx.config`,
  `ctx.mailer`, `ctx.storage`, `ctx.cache`, `ctx.queue_provider`. Don't create
  your own DB pool, server, or job queue.
- Start every controller/model/worker/task with `use loco_rs::prelude::*;`.
- App code returns `loco_rs::Result<T>` and uses `?`.
- Config is YAML in `config/`; secrets come from the environment via the
  `get_env` Tera helper inside the YAML.
- Primary/foreign keys are `i64` (this is Loco 0.17+).
- Tests: `request::<App, _, _>(|request, ctx| async move { ... }).await;`.

## Useful commands

```
cargo loco start            # run the app
cargo loco db migrate       # apply migrations
cargo loco routes           # list routes
cargo loco task <name>      # run a task
cargo loco doctor           # check the environment
```

## Learn more

- Framework agent guide: https://loco.rs/AGENTS.md
- Full single-file reference: https://loco.rs/llms-full.txt
- Docs: https://loco.rs/docs
