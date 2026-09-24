# The Loco doctrine

Loco is Rails for Rust. That is not a tagline — it is the design contract. When
you are unsure how something should work in Loco, the answer is almost always
"the way Rails does it." Where Loco diverges, it diverges because Rust forced
it, never because Loco disagreed with Rails.

This file tells you what good Loco code looks like. Read it before writing any.

---

## What Loco inherits from Rails, unchanged

**Convention over configuration.** File layout, naming, and wiring are fixed.
`src/models/users.rs`, `src/controllers/auth.rs`, `src/mailers/auth.rs`,
`migration/src/mYYYYMMDD_HHMMSS_name.rs`. Do not invent a layout. Do not add a
`services/` or `repositories/` or `utils/` directory — Rails does not have them
and neither does Loco. If you feel the need for one, the code belongs on a model.

**The menu is omakase.** Loco ships an ORM, a queue, a scheduler, a mailer, a
task runner, storage, caching, and a test harness. They are chosen for you and
they are wired together for you. Adding a crate to do something Loco already
does is the single most common way agent-written Loco code goes wrong.

**Fat model, slim controller.** Domain logic lives on the model. The controller
parses input, calls one or two model methods, and renders.

**Integrated systems over decoupled purity.** There is no repository layer, no
service objects, no DTO-mapping ceremony between "domain" and "persistence."
The model *is* the domain object and it knows how to persist itself. That is
Active Record, on purpose.

---

## Where Rust forced a divergence — and what it looks like

This is the part you must internalize, because these are the places where
guessing "what would Rails do" produces code that does not compile, and guessing
"what would idiomatic Rust do" produces code that fights the framework.

### 1. No `method_missing` → you write the finders yourself

Rails synthesizes `User.find_by_email(...)` at runtime. Rust has no such thing,
so Loco's convention is that you **write it explicitly on `impl Model`**. The
doctrine is unchanged — a named finder on the model — only the implementation is
manual.

```rust
impl Model {
    /// # Errors
    /// Returns `ModelError::EntityNotFound` if no such user exists.
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        users::Entity::find()
            .filter(model::query::condition().eq(users::Column::Email, email).build())
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }
}
```

A raw `Entity::find().filter(...)` inside a controller is the fat-controller
smell. It means a finder that should exist on the model does not.

### 2. Ownership → state transitions consume `self` and return the persisted `Model`

Rails mutates in place: `user.update!(verified_at: Time.now)`. Rust's ownership
model makes in-place mutation of a persisted row awkward, so Loco's convention is
a **consuming transition on `impl ActiveModel`** that returns the new `Model`:

```rust
impl ActiveModel {
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::Set(Some(Local::now().into()));
        Ok(self.update(db).await?)
    }
}
```

Called as `user.into_active_model().verified(&ctx.db).await?`. Same doctrine as
Rails — a named domain operation on the model, not a field poke in a controller.
Ownership just made it a consuming method instead of a mutating one.

### 3. No autoloading → the generators are not optional

Rails autoloads `app/models/*.rb`. Rust requires explicit `mod` declarations,
route registration, and worker registration in `src/app.rs`. That wiring is
mechanical, easy to get subtly wrong, and there is a generator for every piece
of it. **This makes `cargo loco generate` more important in Loco than `rails g`
is in Rails, not less.** Generate first, then edit the generated file.

Never hand-edit `src/models/_entities/` — it is regenerated from the database
schema and your edits will be destroyed. Your code goes in `src/models/<name>.rs`,
which re-exports from `_entities`.

### 4. No inheritance → `ApplicationController` becomes middleware + extractors

Rails puts cross-cutting behavior in a base class: `before_action :authenticate`.
Rust has no class inheritance, so Loco splits that into two mechanisms:

- **Cross-cutting, per-request infrastructure** (CORS, compression, timeout,
  logging, limits) is **middleware**, configured in `config/*.yaml`.
- **Per-handler requirements** (is this user logged in?) are **extractors** in
  the handler signature: `auth: auth::JWT`. The type system enforces it — a
  handler that takes `auth::JWT` cannot run unauthenticated.

Middleware order is **LIFO**: the last middleware added is the first to meet the
outside world. This is the opposite of the naive reading and it is the single
most common middleware mistake.

### 5. Exceptions → `Result`

Rails raises and a rescue handler turns it into a 500. Loco returns
`loco_rs::Result<T>` and `?` propagates. Consequences:

- Handlers return `Result<Response>`. Use `?`. Never `.unwrap()` or `.expect()`
  in a handler or worker — a panic there is a crashed request, not a 500.
- `ModelError` converts into `Error` automatically, so a model call inside a
  handler is just `.await?`.
- Return the framework's typed errors (`Error::NotFound`,
  `Error::Unauthorized`, `unauthorized(...)`, `bad_request(...)`) so the right
  status code comes out. A stringly-typed `Error::string("not found")` returns
  500 and is wrong.

### 6. Mixins → traits

Rails uses `include Concern`. Loco uses traits, and the important ones are:

| Trait | Rails ancestor | Purpose |
|---|---|---|
| `Validatable` | `validates :email, ...` | declarative validation on `ActiveModel` |
| `ActiveModelBehavior` | `before_save` callbacks | lifecycle hooks; call `self.validate()?` here |
| `Authenticable` | Devise's user contract | lets the auth middleware find your user |
| `Hooks` | `config/application.rb` | app-level wiring: routes, workers, tasks, seeds |

### 7. Ruby's runtime config → typed, declarative YAML

Rails initializers are arbitrary Ruby. Loco config is `config/<env>.yaml`
deserialized into typed structs, reachable at `ctx.config`. Because it is
declarative and typed, the rule is stricter than in Rails: **application code
does not read `std::env::var`.** If a value is configurable, it belongs in
`config/*.yaml`. Secrets come in via config with env interpolation, not via
ad-hoc env reads scattered through handlers.

---

## Task vs. Worker vs. Scheduler

Rails answers this cleanly and Loco copies the answer exactly. Getting this
wrong is the most common structural mistake in generated Loco code.

| Loco | Rails ancestor | Triggered by | Runs in | Use when |
|---|---|---|---|---|
| **`Task`** | `rake task` | a human, at a CLI: `cargo loco task <name>` | its own process, runs once, exits | one-off or operational work: backfills, reports, imports, admin surgery |
| **`BackgroundWorker`** | Active Job | your code, during a request: `perform_later` | a queue worker process | the request must return before the work finishes: email, thumbnails, webhooks |
| **Scheduler** | `whenever` / cron | the clock, per `config/scheduler.yaml` | **shells out** to a task or command | something must happen on a wall-clock schedule |

The critical structural fact: **the scheduler does not contain work.** Its `Job`
struct holds a `run: String` that is "a task name and also task arguments," and
it executes it as a subprocess. So recurring work in Loco is always *two*
pieces — a `Task` that does the thing, and a scheduler entry that runs that task
on a cron expression. Exactly like `whenever` generating a crontab that calls
`rake`.

Decision procedure:

- Does a user's HTTP request need to return before this finishes? → **Worker**.
- Does it need to happen every night at 3am? → **Task**, plus a **scheduler**
  entry pointing at it.
- Will a human run it by hand, once or occasionally? → **Task**.
- Is it "every night at 3am, and it's slow"? → still a **Task** on a schedule.
  The scheduler runs it in its own process; it does not need to be a worker.

`tokio::spawn` is never the answer. It is not durable, not retried, not
observable, and dies with the process. If you find yourself reaching for it, you
want a `BackgroundWorker`.

---

## The principles, operationally

Check your own output against these before you finish.

### P1 — Use the batteries

Loco ships the infrastructure. Reaching for an external crate, or hand-writing
infrastructure, when Loco ships the capability is the single most common failure
mode in agent-written Loco code.

| You need | Use | Not |
|---|---|---|
| send email | a mailer in `src/mailers/`, `AuthMailer::send_welcome(&ctx, &user)` | `lettre`, a hand-built SMTP client |
| hash a password | `loco_rs::hash::hash_password` / `verify_password` | `argon2`, `bcrypt`, `rand` directly |
| run work out-of-band | `BackgroundWorker` + `perform_later` | `tokio::spawn` |
| retry a failing job | the queue's retry configuration | a hand-rolled loop with `tokio::time::sleep` |
| store a file | `ctx.storage` | `std::fs`, an S3 SDK |
| cache a value | `ctx.cache` | a `HashMap` in a `OnceLock` |
| a unique public id | the generated `pid` (`Uuid`) | your own id scheme |
| paginate | `model::query::PaginationQuery` | manual `LIMIT`/`OFFSET` arithmetic |
| validate | `Validatable` | `if` checks in a handler |
| read a setting | `ctx.config` | `std::env::var` |

Reinvention does not always show up in `Cargo.toml`. A hand-rolled retry loop
adds no dependency and is still a P1 violation.

### P2 — Fat model, slim controller

Finders and creation on `impl Model`. State transitions on `impl ActiveModel`,
consuming `self`, returning the persisted `Model`. The handler orchestrates:

```rust
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    let user = users::Model::create_with_password(&ctx.db, &params).await?;
    AuthMailer::send_welcome(&ctx, &user).await?;
    OnboardingWorker::perform_later(&ctx, OnboardingWorkerArgs { user_id: user.id }).await?;
    let user = user.into_active_model().set_onboarding_started(&ctx.db).await?;
    format::json(UserResponse::from(&user))
}
```

If `src/models/<name>.rs` is nothing but `impl ActiveModelBehavior for ActiveModel {}`
while the controller is 80 lines, the design is inverted.

### P3 — Validation and invariants live on the model

`Validatable` for field rules. `ActiveModelBehavior::before_save` to enforce them
(`self.validate()?`) and to fill derived fields (`pid`, `api_key`). Do not
duplicate a rule inline in a handler that the model already enforces.

### P4 — Config, not environment

`config/<env>.yaml` → `ctx.config`. No `std::env::var` in application code.

### P5 — Respect the framework's contracts

- A multi-statement invariant needs a transaction: `db.begin()` … `txn.commit()`.
  A check-then-insert without one is a race, not a validation.
- Never serialize an entity straight to the client. `format::json(user)` on a
  `users::Model` leaks the password hash and API key. Render a view/DTO from
  `src/views/`.
- Use `loco_rs::prelude::*`. If you are importing `axum::` types directly in a
  controller, you are probably re-deriving something the prelude gives you.

### P6 — Generate, then edit

`cargo loco generate model|scaffold|controller|worker|mailer|task|migration`.
The generator writes the file *and* the wiring. Hand-written wiring is where
"the code exists but is never reached" bugs come from.

---

## The smells, ranked

If you produce any of these, the code is not idiomatic Loco:

1. An entity serialized directly to a response (leaks credentials).
2. `tokio::spawn` for background work.
3. A new dependency for something in the P1 table.
4. Business logic or raw queries in a handler while the model file is empty.
5. `std::env::var` in application code.
6. A hand-rolled retry/backoff loop.
7. Check-then-insert with no transaction.
8. Inline `if` validation duplicating a `Validatable` rule.
9. Edits inside `src/models/_entities/`.
10. `.unwrap()` / `.expect()` in a handler or worker.
