# From a request to running code

The order of operations. `doctrine.md` tells you what good Loco looks like; this
tells you how to produce it without burning turns.

---

## Every change has the same shape

```
generate  →  migrate  →  edit the generated file  →  prove it is wired  →  test
```

Skipping step 1 is the most expensive mistake available to you. Rust has no
autoloading: a file you hand-write is not a module, a handler you hand-write is
not a route, a worker you hand-write is never registered. The code compiles and
is never reached, and the symptom — "my endpoint 404s" — sends you looking in
the wrong place for a long time.

The generator writes the file **and every piece of wiring it needs**. For
`cargo loco generate worker mailer_retry` that is:

| It writes | Where |
|---|---|
| the worker | `src/workers/mailer_retry.rs` |
| the module declaration | `pub mod mailer_retry;` appended to `src/workers/mod.rs` |
| the registration | `queue.register(...)` injected into `src/app.rs` after `fn connect_workers` |
| a test | `tests/workers/mailer_retry.rs` |
| the test's module declaration | appended to `tests/workers/mod.rs` |

You would have had to know about all five. Every generator does this.

---

## Decomposing an English request

Read the request and name the pieces before touching the CLI. Most requests are
a combination of these, and each maps to exactly one generator:

| The user says | You need | Generator |
|---|---|---|
| "posts have a title and a body" | a model + migration | `generate model` |
| "…and I want the usual CRUD endpoints for them" | model + migration + controller + views + tests | `generate scaffold` |
| "add an endpoint that does X" | a controller | `generate controller` |
| "add a column to posts" | a migration | `generate migration` |
| "send them an email when X" | a mailer | `generate mailer` |
| "…and the request shouldn't wait for it" | a background worker | `generate worker` |
| "every night at 3am, do X" | a **task**, plus a scheduler entry | `generate task`, then `generate scheduler` |
| "I need to backfill / import / run something by hand" | a task | `generate task` |
| "only logged-in users can do this" | nothing — an extractor in the handler | (see `recipes/auth.md`) |
| "remember this value so we don't recompute it" | nothing — `ctx.cache` | (see `doctrine.md` P1) |
| "make this configurable" | nothing — `config/*.yaml` → `ctx.config` | (never `std::env::var`) |

The last three matter as much as the first seven: the correct answer is often
*no new file*, because Loco already ships the capability. Check the P1 table in
`doctrine.md` before you add anything.

**Recurring work is always two pieces.** The scheduler holds no work — its
entries shell out to a task by name. So "every night at 3am" means a `Task` that
does the thing, plus an entry under `scheduler:` in `config/<env>.yaml` that runs it. Writing the
logic inside a scheduler entry is not possible.

---

## The generators

```sh
cargo loco generate model <name> <field:type>...
cargo loco generate scaffold <name> <field:type>... --api|--html|--htmx
cargo loco generate migration <Name> [field:type]...
cargo loco generate controller <name> [actions]... [--api|--html|--htmx]
cargo loco generate task <name>
cargo loco generate worker <name>
cargo loco generate mailer <name>
cargo loco generate scheduler
cargo loco generate data <name>
cargo loco generate deployment docker|nginx|lambda
```

Model names are **plural** (`posts`, `comments`) — the generator derives the
singular struct name from the plural table name, the way Rails does.

`scaffold` is `model` + `controller` + views + tests in one shot. Use it when
the user wants CRUD; use `model` + `controller` separately when the endpoints
are not CRUD-shaped.

### The column DSL

`name:spec`, repeated. The spec grammar:

| Spec | Meaning |
|---|---|
| `title:string` | **nullable** — bare is nullable |
| `title:string!` | `NOT NULL` |
| `email:string^` | unique **and** `NOT NULL` |
| `user:references` | 64-bit foreign key, **`NOT NULL`** |
| `user:references?` | nullable foreign key |
| `user:references:admin_id` | foreign key with a custom column name |
| `status:enum:draft,live,archived` | enum, `!`/`^` allowed |
| `price:decimal_len:10:2` | precision and scale |
| `tags:array:string` | array; inner is `string`/`int`/`big_int`/`float`/`double`/`bool` |

**The one inconsistency worth memorising:** bare scalars are nullable, but bare
`references` is `NOT NULL`. `references?` is the nullable one. This is
deliberate and it will catch you.

`^` implies `!` — a unique column is always required, so `string^` is both. The
generator rejects `^` on types with no btree index: `bool`, `tstz`, and `json`
(use `jsonb`).

Scalar type names, complete:

`string` `text` `uuid` `bool` `date` `time` `date_time` `tstz` `json` `jsonb`
`blob` `money` `decimal` `float` `double` `small_int` `small_unsigned`
`unsigned` `big_unsigned` `int` `big_int`

Every model gets an `id` primary key automatically. Do not declare one.

### After `generate model` or `generate migration`

```sh
cargo loco db migrate      # apply it
cargo loco db entities     # regenerate src/models/_entities/ from the new schema
```

Both, in that order, every time. `_entities/` is derived from the live database
schema — until you migrate and regenerate, your model does not have the fields
you just declared, and the compile errors will not say so.

---

## Continuing where the generator stopped

The generator gives you a correct, empty-bodied skeleton. Your work goes
**inside the files it created**, never alongside them.

| After | The file to edit | What goes in it |
|---|---|---|
| `generate model posts` | `src/models/posts.rs` | finders on `impl Model`, state transitions on `impl ActiveModel`, `Validatable`, `before_save` |
| `generate controller` | `src/controllers/<name>.rs` | handler bodies; add routes to the existing `routes()` |
| `generate worker` | `src/workers/<name>.rs` | fill `WorkerArgs`, write `perform` |
| `generate task` | `src/tasks/<name>.rs` | write `run`; read args from the `Vars` it hands you |
| `generate mailer` | `src/mailers/<name>.rs` + its `templates/` | the send method and the html/text/subject templates |
| `generate scaffold` | model **and** controller | logic on the model; leave handlers thin |

Two rules that follow from this:

- **Never create a file the generator would have created.** If you find yourself
  writing `src/workers/foo.rs` by hand, stop and run the generator.
- **Never hand-write a `mod` line, a route registration, or a
  `queue.register(...)`.** If one is missing, the generator did not run, and
  adding it by hand hides that fact instead of fixing it.

The generator refuses to overwrite an existing file (`skip_exists`). If you
generated something with the wrong name, delete the file *and* its injected
lines rather than generating a second one alongside it.

---

## Proving it is wired

```sh
cargo loco routes          # every registered route. Your endpoint IS here or it is not served.
cargo loco task --list     # every registered task
cargo loco scheduler --list # every scheduled job
cargo loco doctor          # config and DB reachability
```

Run `cargo loco routes` after adding any endpoint. It is the difference between
"the handler exists" and "the handler runs", and it costs one second.

---

## Before you call it done

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

All three. `clippy -D warnings` is not optional style — a Loco app is expected
to be clean under it, and a warning there is frequently a real defect (an
unawaited future, an ignored `Result`).

Then check your own output against the ranked smell list at the end of
`doctrine.md`. The top three account for most of what goes wrong: an entity
serialized straight to a response, `tokio::spawn` instead of a worker, and a new
dependency for something in the P1 table.
