---
title: Your First App
description: Install Loco, generate a new app, scaffold a CRUD resource, and hit your first endpoint — a guaranteed-success first run.
sidebar:
  order: 1
---

This is the fastest path from "nothing installed" to "a working API you built yourself." You'll install the tooling, generate a new Loco app, start it, talk to it with `curl`, then add a database-backed resource with a single generator command. Every step below is meant to work exactly as written — if something doesn't match what you see, that's worth reporting.

You need a working Rust toolchain (stable, via [rustup](https://rustup.rs)) and about 10 minutes. No prior Loco knowledge is assumed.

## 1. Install the tooling

Loco ships as two things: the `loco` app generator (a small standalone CLI), and `loco-rs`, the framework your generated app depends on. You also need `sea-orm-cli` because your app will use a database.

```sh
cargo install loco
cargo install sea-orm-cli
```

## 2. Generate a new app

Run `loco new` with the app name and the database, background-worker, and asset flags spelled out explicitly. With all four supplied there is nothing left to ask, so the wizard skips every prompt — a fully deterministic, scriptable way to create an app:

```sh
loco new --name hello_loco --db sqlite --bg async --assets none
```

```sh
🚂 Loco app generated successfully in:
hello_loco/
```

<div class="infobox">
If the directory you run this in is itself inside a git repository, <code>loco new</code> asks you to confirm before continuing. Answer <code>y</code>, or pass <code>-a</code>/<code>--allow-in-git-repo</code> to skip the prompt entirely.
</div>

You now have a `hello_loco/` folder with a runnable app inside it. Here's the part of the layout you'll touch in this lesson:

| Path | What's there |
|---|---|
| `src/app.rs` | Wires routes, workers, and tasks together — the one file that ties everything to `Hooks`. |
| `src/controllers/` | Request handlers, one file per resource. |
| `src/models/` | Your database entities (`_entities/`, generated) and your own model logic. |
| `migration/src/` | One file per schema change, applied in order. |
| `config/development.yaml` | Settings for the `development` environment — port, database URI, logging, etc. |

`--db sqlite` picked SQLite (a local file, zero setup) as the database, `--bg async` runs background jobs in-process, and `--assets none` skips generating server- or client-rendered view scaffolding — you're building a pure JSON API.

## 3. Start the server

```sh
cd hello_loco
cargo loco start
```

You'll see Loco's boot banner and, at the bottom, `listening on port 5150`. `cargo loco` is not a real cargo subcommand — it's a Cargo alias (`loco = "run --"`) baked into every generated app's `.cargo/config.toml`, so `cargo loco start` really runs your app's own binary with `start` as an argument.

Leave this running and, in another terminal, hit the built-in liveness check — no code written yet, and it already answers:

```sh
$ curl localhost:5150/_ping
{"ok":true}
```

`/_ping` is one of three built-in monitoring endpoints (`/_ping`, `/_health`, `/_readiness`) mounted unconditionally by `AppRoutes::with_default_routes()` in `src/app.rs`.

<div class="infobox">
Because you picked a database (<code>--db sqlite</code>), this app was also generated with a complete, ready-to-use authentication suite mounted at <code>/api/auth/*</code> (register, login, current user, and more) — any Loco app with a database gets one, it isn't specific to a particular starter "template". This lesson doesn't use it; if you want to explore it, see <a href="/docs/tutorials/saas-with-auth">Build a small authenticated app</a>.
</div>

Stop the server with `Ctrl+C` before continuing — you'll restart it after generating code.

## 4. Generate a CRUD resource

This is where Loco earns its keep. A **scaffold** generates a database migration, a Sea-ORM model/entity, typed request/response DTOs, and a full CRUD controller — in one command:

```sh
cargo loco generate scaffold posts title:string content:text --no-auth
```

The scaffold is **adaptive** — no kind flag to pick. In a headless app (like this one) it produces a JSON API controller; if your app has a React frontend (`frontend/`), it also emits typed hooks and pages for the resource. The output ends with a few confirmation lines:

```sh
* Migration for `posts` added! You can now apply it with `$ cargo loco db migrate && cargo loco db entities`.
* A test for model `Posts` was added. Run with `cargo test`.
* Controller `Post` was added successfully.
* DTO `PostDto` was added successfully.
```

<div class="infobox">
<strong>Why <code>--no-auth</code>?</strong> Scaffolded routes are authenticated by default: every handler takes an <code>auth::JWT</code> extractor, so an anonymous <code>curl</code> gets <code>401 Unauthorized</code>. That's the right default for a real resource, but it would turn this lesson into an auth tutorial. <code>--no-auth</code> generates the same controller with public routes. To see the authenticated flavor — register, log in, send the bearer token — follow <a href="/docs/tutorials/saas-with-auth">Build a small authenticated app</a>.
</div>

Unlike a plain `migration` generator, `scaffold` (like `model`) already **applied** the migration and regenerated the Sea-ORM entities for you — there's nothing left to run manually. You should now have:

```
src/
  controllers/posts.rs      <- CRUD handlers + routes
  dtos/posts.rs             <- request/response types
  models/_entities/posts.rs <- generated Sea-ORM entity
  models/posts.rs           <- your extension point
migration/
  src/mYYYYMMDD_HHMMSS_posts.rs
```

`title:string` and `content:text` are both nullable columns here (no `!`/`^` suffix) — that's intentional to keep this first pass simple. The field-type suffixes (required, unique) and the full type list are covered in [Generators & field types](/docs/reference/generators).

## 5. Run it and hit your new endpoint

```sh
cargo loco start
```

In another terminal, create a post:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "My first Loco post",
  "content": "It works."
}' localhost:5150/api/posts

{"id":1,"title":"My first Loco post","content":"It works.","created_at":"...","updated_at":"..."}
```

And list it back:

```sh
$ curl localhost:5150/api/posts
{"items":[{"id":1,"title":"My first Loco post","content":"It works.","created_at":"...","updated_at":"..."}],"page":1,"page_size":25,"total_pages":1,"total_items":1}
```

The list endpoint is paginated, so it answers with a page envelope rather than a bare array. `page` and `page_size` are query parameters — `curl 'localhost:5150/api/posts?page=2&page_size=10'` — and the metadata field names are the same ones the framework's own [pagination helpers](/docs/how-to/paginate) use.

That's a full round trip: a generated migration created the `posts` table, a generated Sea-ORM entity modeled it, and a generated controller exposed it over HTTP — with zero hand-written Rust.

## What you built

In a few minutes, without writing a line of Rust yourself, you:

- Installed the Loco CLI and `sea-orm-cli`.
- Generated a new app with an explicit, reproducible `loco new` command.
- Started it and hit two built-in endpoints (`/api`, `/_ping`).
- Generated a complete CRUD API for a `posts` resource and exercised it with `curl`.

## Next

- [The Tour](/docs/tutorials/the-tour) — a faster walkthrough that also covers models with relations, hand-editing a controller, background workers, and tasks.
- [Add a model](/docs/how-to/add-model) — the how-to version of what you just did, with the field-type mini-language spelled out.
- [Build a small authenticated app](/docs/tutorials/saas-with-auth) — start from the SaaS starter path instead, with registration, login, and JWT-protected routes baked in.
- [CLI reference](/docs/reference/cli) and [Generators & field types](/docs/reference/generators) — the exhaustive dictionaries behind everything you just ran.
