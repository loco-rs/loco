# When the compiler is angry

A map from symptom to cause. Loco's failure modes are narrow and repetitive —
almost every compile error you will hit in a Loco app is one of the entries
below. Look here before you start changing code, because the wrong guess costs
several turns and the error messages point away from the real cause.

---

## The stop rule

**If you have made three attempts at the same error and it has not moved, the
cause is not what you think it is.** Stop editing. Do one of these instead:

1. Re-read the exact error text, including the `help:` line — Rust frequently
   prints the fix and it gets skipped.
2. Check `api-index.md` / `sea-orm-index.md` for the symbol you are calling. If
   it is not there, it does not exist, and no amount of editing will help.
3. Read the working example in the starter app (`starter-app.md` has the map).

Repeatedly re-editing the same call is the single largest waste of turns in a
Loco session, and it is nearly always one of the two causes below: a missing
trait import, or a method that does not exist.

---

## 1. Sea-ORM: "no method named X found" — a trait is not in scope

**By far the most common Loco compile error.** Sea-ORM's query surface is
almost entirely trait methods, so the method you want lives on a trait, not on
the type it appears to belong to. If the trait is not imported, the method is
invisible and the error claims it does not exist.

```
error[E0599]: no method named `load_many` found for struct `Vec<posts::Model>`
help: trait `LoaderTrait` which provides `load_many` is implemented but not in scope
  1 + use sea_orm::LoaderTrait;
```

**The `help:` line is the fix. Read it.** Rust names the missing trait; add
that import rather than changing the call.

`loco_rs::prelude::*` re-exports every Sea-ORM trait needed to write a query,
so in practice you should not hit this while filtering, ordering, paginating,
or building conditions:

| In the prelude — no import needed | Import explicitly |
|---|---|
| `ColumnTrait`, `EntityTrait`, `ModelTrait` | `RelationTrait`, `JoinType` — joins |
| `ActiveModelTrait`, `ActiveModelBehavior`, `IntoActiveModel` | `LoaderTrait` — eager loading related rows |
| `QueryFilter` — `.filter()` | `FromQueryResult` — custom row structs |
| `QueryOrder`, `Order` — `.order_by*()` | `IntoCondition` |
| `QuerySelect` — `.limit()`, `.offset()`, `.column()` | `Iterable` — enumerating `Column` |
| `PaginatorTrait` — `.paginate()`, `.count()` | `ExprTrait` — see below |
| `Condition`, `Expr` | |
| `ConnectionTrait`, `TransactionTrait` | |
| `Set`, `ActiveValue`, `DbErr`, `DatabaseConnection` | |

If a query-building method is missing, check the spelling against
`sea-orm-index.md` before adding an import — the prelude almost certainly
already has the trait.

**`ExprTrait` is the one exception, and do not glob it into a module.** It is
`impl<T> ExprTrait for T`, so `use sea_orm::ExprTrait;` at the top of a file
reaches every value in it: `n.max(1)` stops compiling, and `n.eq(&3)` quietly
returns an `Expr` instead of a `bool` because its receiver is `self` where
`PartialEq::eq` takes `&self`. If you genuinely need it, import it inside the
one function that does. Usually you do not — see the next section.

## 2. You are building an expression when you want a query

Reaching for `Expr::` usually means you have gone one layer too low. Loco ships
a condition builder and Sea-ORM puts the comparisons on the column:

```rust no-syntax-check="expression fragments, shown side by side to contrast layers"
// wrong layer — sea-query expression building
Expr::expr(Func::lower(Expr::col(users::Column::Name))).like(&pattern)

// right — the column's own comparison methods
users::Column::Name.contains(&pattern)      // case-insensitive LIKE %..%
users::Column::Email.eq(&email)
users::Column::Name.starts_with(&prefix)

// or Loco's condition builder, which is what the framework's own models use
model::query::condition().eq(users::Column::Email, email).build()
```

`ColumnTrait` (in the prelude) carries `eq`, `ne`, `gt`, `lt`, `gte`, `lte`,
`like`, `contains`, `starts_with`, `ends_with`, `is_in`, `is_null`, `between`.
Check `sea-orm-index.md` before using anything else.

## 3. "the trait bound ... is not satisfied" on a handler — add `#[debug_handler]`

Axum reports handler signature problems as an unreadable trait-bound wall that
names none of the offending types.

```rust no-syntax-check="a handler signature, deliberately without its body"
#[debug_handler]                    // <- this, always, on every handler
async fn show(State(ctx): State<AppContext>, Path(id): Path<i32>) -> Result<Response> {
```

`debug_handler` is in `loco_rs::prelude`. With it, the error names the actual
problem. Without it you are guessing. The usual underlying causes:

- an extractor that is not last when it consumes the body (`Json<T>`,
  `Form<T>`, `Multipart` must come **last** in the argument list)
- a return type that is not `Result<Response>`
- a parameter type that is not an extractor at all

## 4. "the router state is not inferred" / state type mismatch

Every handler in a `Routes` must take `State(ctx): State<AppContext>` — even
handlers that do not use `ctx`. Axum infers the router's state type from the
handlers; one handler without it and the whole router fails to type-check, with
an error that points at the router rather than the handler.

## 5. "only traits defined in the current crate can be implemented for types defined outside of the crate" (E0117)

You wrote `impl From<SomeLocoType<T>> for YourType` where `YourType` is a type
alias for a foreign type. An alias is not a new type; it is the foreign type.

Define a real struct in `src/views/`, or write a free function that constructs
the response instead of a `From` impl.

## 6. `?` cannot convert `loco_rs::Error` into `ModelError`

```
error[E0277]: `?` couldn't convert the error to `ModelError`
    the trait `From<loco_rs::Error>` is not implemented for `ModelError`
```

Model methods conventionally return `ModelResult<T>`, and the conversion only
goes one way: `Error: From<ModelError>`, never the reverse. So the moment a
model method calls a *framework* helper that returns `loco_rs::Result` — the
pagination helpers are the usual case — `?` stops working.

```rust no-syntax-check="two signatures contrasted; `..` stands in for the argument list"
// does not compile: query::paginate returns Result, the method promises ModelResult
pub async fn search(..) -> ModelResult<query::PageResponse<Self>> {
    Ok(query::paginate(db, Entity::find(), Some(cond), pagination).await?)
}

// right: return the same Result the helper does
pub async fn search(..) -> Result<query::PageResponse<Self>> {
```

`ModelResult` is for methods whose failures are model failures — not found,
already exists, validation. A method that is mostly a call into the framework
should return `Result` and let the caller's `?` do the rest.

**Do not over-apply that.** It is a narrow escape hatch for the framework
helpers, not a new convention:

- A method that only calls **Sea-ORM** — `count`, `find`, `insert` — keeps
  `ModelResult`. `DbErr` converts into `ModelError` cleanly, so `?` works and
  nothing forced your hand.
- A model method takes **`&DatabaseConnection`, never `&AppContext`**. Every
  finder in the starter app does. Reaching for `&ctx` to dodge a signature
  problem drags the queue, mailer, storage, and cache into the model layer to
  run one query.

The only thing that justifies widening a model method's return type is a call
into a framework helper that returns `Result` and has no `ModelError` form.

## 7. Your model does not have the field you just added

You ran `cargo loco generate model` or `generate migration` and the field is
missing from the entity.

```sh
cargo loco db migrate      # apply the migration to the database
cargo loco db entities     # regenerate _entities/ FROM the database
```

`src/models/_entities/` is derived from the live schema, not from the migration
file. Until both commands run, the field does not exist in Rust. Editing
`_entities/` by hand to add it will appear to work and be destroyed on the next
regeneration.

## 8. A private module path that looks public

`api-index.md` is generated from rustdoc and lists real symbols, but a path that
appears in documentation is not necessarily importable. If an import fails with
"module is private", the type is re-exported somewhere else — usually the
prelude or the parent module. Import it from `loco_rs::prelude::*` or from the
parent, not from the definition path.

## 9. The endpoint compiles, and 404s

It is not wired. `cargo loco routes` lists what is actually served; if your path
is not there, the controller's `routes()` was never registered in `src/app.rs`.

This is what generating instead of hand-writing prevents. The fix is not to add
the registration by hand — it is to check why the generator did not run.

The same failure shape applies to workers (never registered in `connect_workers`,
so `perform_later` goes nowhere) and tasks (absent from `cargo loco task --list`).

---

## Failures that are not your code

Rule these out before debugging a generated app — all of them produce errors
that look like defects in what you wrote:

| Symptom | Cause |
|---|---|
| A fresh app fails to compile with type errors | The installed `loco` CLI is older than the `loco-rs` version being depended on |
| DB tests fail with a socket/connection error | No database reachable. `cargo loco doctor` says so directly. |
| Build dies with `killed` / `stopped` and no error | Out of memory. Rebuild with fewer jobs. |
| `couldn't read .../out/bindgen.rs: No such file` | A previous build was killed mid-compile, leaving a partial target directory |

`cargo loco doctor` checks the environment ones in a single command. Run it
before assuming the code is wrong.
