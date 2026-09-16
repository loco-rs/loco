# The app you are standing in

`loco new` does not produce an empty project. It produces a working app with
authentication, a user model, mailers, example tasks, an example worker, and a
passing test suite. **Most of what you are about to be asked to build already
has a working example in the tree.**

Read this before exploring the app by hand. Exploring costs turns and this file
is exact.

---

## Setting up a new app

Five templates. Picking one answers between one and three follow-up questions:

| Template | DB | Background | Assets |
|---|---|---|---|
| Saas App with server side rendering | asks | asks | **serverside** (pinned) |
| Saas App with client side rendering | asks | asks | **clientside** (pinned) |
| Rest API (with DB and user auth) | asks | asks | **none** (pinned) |
| lightweight-service | **none** (pinned) | **async** (pinned) | **none** (pinned) |
| Advanced | asks | asks | asks |

The three answers, with their CLI flag values:

| Flag | Values | Meaning |
|---|---|---|
| `--db` | `sqlite` (default), `postgres`, `none` | SQLite is a file, zero setup. Postgres needs a reachable instance. |
| `--bg` | `async` (default), `queue-redis`, `queue-postgres`, `queue-sqlite`, `blocking` | `async` runs jobs in-process — no separate worker process. `queue-*` needs that backend reachable and a `cargo loco start --worker`. `blocking` **blocks the request** until the job finishes; it is for tests. |
| `--assets` | `serverside` (default), `clientside`, `none` | `serverside` = Tera templates in `assets/`. `clientside` = a React SPA in `frontend/`. `none` = pure JSON API. |

**Supplying all three flags skips every prompt.** Add `--name` and the run is
fully non-interactive, which is how you should always invoke it:

```sh
loco new --name blog --db sqlite --bg async --assets none
```

`--embedded-assets` bakes static files into the binary. Serverside only — it is
**silently ignored** for clientside and none.

### What to ask the user before running it

You need exactly three facts, and there are good defaults for all of them. Ask
in this order, and stop asking as soon as the answers are determined:

1. **What kind of app** — JSON API, server-rendered pages, or a React frontend?
   That picks the template and pins `--assets`.
2. **Which database** — SQLite unless they say otherwise, or they mention
   deploying somewhere with a managed Postgres. `none` only if they explicitly
   want no persistence.
3. **Background work** — `async` unless they need jobs to survive a restart or
   to run on separate machines, which means a `queue-*` backend.

Do not ask about anything else. Auth, mailers, and the test harness are not
choices — they come with a database.

---

## What you land in

With a database (`--db sqlite|postgres`), which is the common case:

```
src/
  app.rs                    Hooks impl — routes, workers, tasks, initializers are registered HERE
  controllers/auth.rs       register, login, verify, forgot, reset, magic-link — all working
  views/auth.rs             the DTOs those handlers render
  models/
    _entities/users.rs      GENERATED from the schema — never edit
    users.rs                the users model: finders, password handling, validation
  mailers/auth.rs           welcome / forgot / magic_link, with html+text+subject templates
  tasks/user_create.rs      a working Task
  tasks/user_delete.rs      a second working Task
  workers/downloader.rs     a working BackgroundWorker
  dtos/common.rs            shared response types
  fixtures/users.yaml       seed data
migration/src/
  m20220101_000001_users.rs the users table
tests/
  requests/auth.rs          request tests against the auth endpoints
  models/users.rs           model tests, with insta snapshots
config/{development,test,production}.yaml
```

Conditional pieces:

| Condition | You also get |
|---|---|
| `--assets serverside` | `assets/` (Tera views, i18n, static files) and `src/initializers/view_engine.rs` |
| `--assets clientside` | `frontend/` — a Vite + React + react-router SPA with a typed API client, login page, and auth guard |
| `--db none` | **no** models, migration, mailers, fixtures, or auth. You get `src/controllers/home.rs` + `src/views/home.rs` instead of the auth controller. |

Auth and mailers are tied to the database: a db app always has both, a `--db
none` app has neither. There is no flag to separate them.

**`cargo loco start` works immediately.** So does `cargo test`. If either fails
on a freshly generated app, the problem is the environment, not your code.

---

## The starter app is your best reference

Before reading a recipe, read the working example that is already on disk. It
compiles, it is idiomatic by construction, and it is shorter than prose.

| Writing this | Read this first |
|---|---|
| a model with finders and validation | `src/models/users.rs` |
| a migration | `migration/src/m20220101_000001_users.rs` |
| a controller with routes and params | `src/controllers/auth.rs` |
| a response DTO | `src/views/auth.rs` |
| a `Task` | `src/tasks/user_create.rs` |
| a `BackgroundWorker` | `src/workers/downloader.rs` |
| a mailer, with templates | `src/mailers/auth.rs` + `src/mailers/auth/welcome/` |
| a request test | `tests/requests/auth.rs` |
| a request test that needs a logged-in user | `tests/requests/prepare_data.rs` — `init_user_login` and `auth_header` are already written; do not roll your own |
| a model test with fixtures | `tests/models/users.rs` |
| registering anything | `src/app.rs` |

`src/app.rs` is the single most important file to read. Every route, worker,
task, and initializer in the app is registered there, and it shows you the exact
shape your own registration must take.

---

## What the users model already gives you

Do not rebuild any of this:

- password hashing and verification
- a `pid` (UUID) for every user, generated on save
- an `api_key`
- email verification tokens and timestamps
- password reset tokens with expiry
- magic-link tokens
- `Validatable` rules, enforced in `before_save`

If a request involves users, extend `src/models/users.rs` rather than creating a
parallel model.

---

## First moves after `loco new`

```sh
cd <app>
cargo loco doctor          # confirms DB reachability and config before you write anything
cargo loco routes          # what is already served
cargo test                 # the baseline is green; keep it that way
```

Run `cargo loco routes` again after every generator. It is the cheapest proof
that what you generated is actually reachable — a route that does not appear
there is not wired, no matter what the file contains.
