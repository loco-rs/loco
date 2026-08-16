# Changelog

## 1.1.0 - 2026-08-15

Moves the template engine to Tera 2, makes configuration files valid YAML,
opens up the storage API, and adds an AWS Lambda deployment target.

**Upgrading:** for most apps this is a one-line change — bump
`fluent-templates` from `0.13` to `0.15` in `Cargo.toml`. That crate supplies
the `t()` i18n function in generated apps and pinned Tera 1; 0.15 moved to
Tera 2. Verified by generating an app and compiling it with no other edit.

Beyond that, only apps that register **their own** Tera filters or functions
need work (see the breaking note below), and only templates using
`{% macro %}`/`{% import %}`, `v.0` array access, or relying on undefined
variables rendering empty need editing — Tera 2 replaced macros with
components, requires `v[0]`, and errors on undefined variables.

That last one applies to **mailer** templates as well as views: they render
through the same Tera 2 instance, so a mail template referencing an optional
field that is sometimes absent now fails at send time where Tera 1 rendered
an empty string. Add `| default(value="")` to those references.

### Breaking

- **Scaffolded `list` endpoints changed their JSON contract**, and now use the
  framework's own pagination instead of a second, parallel one. The scaffold
  had its own `ListParams`, its own paginator arithmetic, and an envelope with
  no `total_pages` — while `query::PaginationQuery` + `query::paginate` already
  existed and carried it. The generated handler is now four lines over
  `query::paginate`, and `Page<T>` (`src/dtos/common.rs`) is built by
  `Page::from_query`.

  On the wire: `per_page` → `page_size`, `total` → `total_items`, and
  `total_pages` is added — the metadata field names are `PagerMeta`'s, so an
  app has one pagination vocabulary whichever envelope a handler returns. The
  query parameter is likewise `page_size`. Existing apps keep the code they
  already generated; regenerating, or updating by hand, is a rename plus one
  new field. The typed frontend's `bindings/Page.ts` regenerates from `ts-rs`.

- **`QueueProvider` gains a required `retry_failed` method.** Custom queue
  provider implementations must implement it; all three built-in drivers do.
  (`Queue::retry_failed` is the inherent forwarding method on the handle and
  requires nothing of anyone.)

- **Storage `StoreDriver` / `StorageStrategy` gain required `list` and `stat`
  methods** (and `StorageStrategy` also requires `exists`). Custom driver and
  strategy implementations must implement them. Built-in drivers/strategies
  already do. The `Storage` facade exposes matching `exists` / `list` / `stat`
  (and `*_with_policy`) APIs alongside the existing upload/download/delete
  surface. Under `ReplicatedStrategy` in mirror mode, `exists` and `list` treat
  a miss (`false` / `[]`) the way the other reads treat an error and fall back
  to secondaries; backup mode stays primary-only.
  ([#1805](https://github.com/loco-rs/loco/pull/1805))

- **The template engine is now Tera 2.** `tera::Tera` appears in Loco's public
  API (`PostProcessFn`, `HotReloadingTeraEngine::engine`, `TeraView::tera`,
  `Error::Tera`, `register_filters`), so custom filters and functions must move
  to Tera 2 signatures: filters now take `(Arg, Kwargs, &State)` over Tera's own
  `Value` rather than `(&Value, &HashMap)` over `serde_json::Value`.

  Loco absorbs the rest. Tera 2 dropped `get_env` — which every Loco config
  depends on — so Loco registers its own with Tera 1 semantics. View loading
  moved to `tera::load_from_glob`, which yields the same template names Tera 1
  produced, so existing `render("home/hello.html")` calls are unaffected.

- **Newly generated apps require their production secrets to be set.** The
  generated `config/production.yaml` takes no defaults for secrets or
  addresses, so a missing one stops the app at startup with the variable's
  name rather than falling back to a development value. Always required:
  `DATABASE_URL`, `JWT_SECRET`, `HOST`. Required too, when the app was
  generated with the corresponding component: `REDIS_URL` / `QUEUE_URL` for a
  Redis or database-backed queue, and `MAILER_HOST` / `MAILER_USER` /
  `MAILER_PASSWORD` for a mailer. Existing apps are unaffected — their config
  files are their own. See *Fixed* below.

- `doctor::Resource` is now `#[non_exhaustive]` and has gained a
  `ProductionSafety` variant. The variant is **first**, and `Resource` is the
  key type of the `BTreeMap` the doctor report is built from, so its derived
  `Ord` sets the report's display order — production safety now leads, and
  every other variant shifts by one. Code comparing or sorting `Resource`
  values sees the new order.

- **The three built-in number filters take Tera 2 signatures.**
  `views::tera_builtins::filters::number::{number_with_delimiter,
  number_to_human_size, number_to_percentage}` are `(value, Kwargs, &State)`
  rather than `(value, &HashMap)`. They are `pub`, so calling them directly —
  rather than through a template — needs updating. Templates are unaffected.

- **`auth.jwt.location` is parsed strictly.** Its `Deserialize` is hand-written
  rather than `#[serde(untagged)]`, so it accepts exactly the documented shapes
  — a map, or a list of maps — and reports a real error for anything else
  instead of a generic "did not match any variant". Config that happened to
  parse through untagged fallback will now be rejected with a message naming
  the problem. The serialized form is unchanged.

- `bgworker::pg::get_jobs` returned `Result<Vec<Job>, sqlx::Error>` where the
  SQLite driver's returned `loco_rs::Result`. It now returns `loco_rs::Result`
  too. Callers using `?` in a Loco context are unaffected; code matching on
  `sqlx::Error` needs updating.

- `loco_gen::AppInfo` gains a `working_dir` field, so post-generation checks
  read the tree that was generated into rather than the process's current
  directory. Callers using `new_generator()` should pass `".".into()`.

- `loco_gen::Component::Scaffold` and `::Controller` gain a required `auth:
  bool` field, backing the new `--no-auth` / `--auth` flags. Code constructing
  these variants directly — rather than going through the CLI — must supply it.

- `loco_gen::DeploymentKind` and the CLI's `DeploymentKind` both gain a
  `Lambda` variant. Neither is `#[non_exhaustive]`, so an exhaustive `match`
  over either stops compiling until the arm is added.

- **`post_process` now runs before templates are loaded**, in both the
  on-disk and embedded view engines. A `build_with_post_process` closure can
  no longer inspect loaded templates — it registers into an empty engine — and
  anything a template calls must be registered by that closure, or template
  loading fails. This is what lets a custom filter be visible to the templates
  that use it; previously registration happened too late.

### Added

- **`--no-auth` on `generate scaffold`, `--auth` on `generate controller`.**
  Scaffolded routes take an `auth::JWT` extractor on all five handlers, which
  is the right default but had no opt-out and nothing said so — the first
  `curl` against a fresh scaffold answered 401 with no explanation, and the
  tutorial's own examples were among the casualties. The scaffold now prints
  which flavor it generated, and `--no-auth` emits public routes. A generated
  controller has no model to protect and stays public by default; `--auth` is
  its opt-in mirror, and it adjusts the generated request test to assert the
  route rejects anonymous callers rather than expecting 200.

  `cargo loco generate` now runs a best-effort `cargo fmt` afterwards, the way
  `loco new` already did. A Tera template cannot know where rustfmt would break
  a line — with the auth argument gone, three handler signatures fit on one —
  and a generated app runs `cargo fmt --check` in its own CI, so non-canonical
  output would have failed a user's build on code they did not write.
- **`cargo loco jobs retry`** moves failed jobs back to `queued` — with
  `--id <ID>` for one, or bare for all of them. No queue driver has automatic
  retry or backoff, so a failed job used to be terminal: `requeue`, the verb
  that sounds like the recourse, only rescues jobs a crashed worker stranded in
  `processing` and cannot touch a failed one. `run_at` is reset so a job that
  failed on a future-dated schedule runs now rather than when that time
  arrives. On the Redis provider a retried job is queued to `default`: the
  queue a job was submitted to is not recorded once it fails. The command says
  so when it retries anything, but operators running multiple named queues
  should know before they need it.
- The `loco` CLI now declares `rust-version = "1.94"`, matching the framework.
  `cargo install loco` on an older toolchain refuses up front with the required
  version instead of failing partway through a build.
- `QueueConfig::dangerously_flush()` is now public, for tests and tooling that
  need to empty a queue outright.
- **AWS Lambda deployment generator** — `cargo loco generate deployment lambda`
  writes a `src/bin/lambda.rs` entrypoint, adds `lambda_http`, and writes a
  `[package.metadata.lambda]` block so `cargo lambda build` and
  `cargo lambda deploy` need no flags. Loco's router is a `tower::Service` and
  so is the Lambda runtime, so the app runs unchanged; deployment is delegated
  to `cargo-lambda` rather than embedding an AWS SDK. HTTP only — workers and
  the scheduler do not fit Lambda's model.
  ([#1699](https://github.com/loco-rs/loco/issues/1699))

- **`AppContext::into_builder`** — the escape hatch `Hooks::after_context` was
  missing. `AppContext` is `#[non_exhaustive]`, so `AppContext { storage, ..ctx }`
  (the idiom the storage how-to showed) does not compile outside `loco-rs`, and
  rebuilding from `AppContext::builder` silently drops the mailer, queue
  provider, cache and shared store that boot had already attached.
  `ctx.into_builder().storage(..).build()` replaces one component and keeps the
  rest.
- **`loco_rs::schema::rename_column`**, alongside `add_column`/`remove_column`.
- **`PageResponse` is now `Serialize`/`Deserialize`.** The pagination how-to
  showed `format::json(res)` returning one straight from a handler, and printed
  the JSON body it produces; that could not compile, because the struct derived
  only `Debug`. The documented body is now pinned by a test.

### Security

- **`opendal` 0.57 → 0.58.1**, which moves `quick-xml` from `^0.39.3` to
  `^0.41.0` in the S3, Azure and GCS service crates. `quick-xml` below 0.41.0
  carries RUSTSEC-2026-0194 (quadratic time checking a start tag for duplicate
  attribute names) and RUSTSEC-2026-0195 (unbounded namespace-declaration
  allocation); both are denial of service, and both are `patched = [">= 0.41.0"]`.

  This reaches only apps that enable a cloud storage feature — `storage_aws_s3`,
  `storage_azure`, `storage_gcp`, all off by default — and the XML being parsed
  is the storage backend's own responses, so exploiting it means controlling
  what that endpoint returns. Narrow, but real if you point Loco at an
  S3-compatible endpoint you do not run.

  **This fix ships in 1.1.0, not in 1.0.x.** The 1.0.0 section below described
  the bump as if it had shipped there; it had not — 1.0.0 and 1.0.1 both went
  out pinning `opendal = "0.57"`. That text has been corrected. If you enabled a
  cloud storage feature on 1.0.0 or 1.0.1, upgrading to 1.1.0 is the fix.

### Fixed

- **On the SQLite queue driver, clearing the queue stopped it handing out jobs
  — permanently and silently.** `dequeue` claimed an advisory lock by updating
  a row in a second table, `sqlt_loco_queue_lock`, while `clear` deleted every
  row in that table. With the row gone the lock could never be acquired, so
  `dequeue` returned "no jobs" forever, with the jobs sitting right there. Any
  app configured with `queue.dangerously_flush: true` hit this on every boot —
  `converge` runs `setup` and then `clear` — so its workers never ran a single
  job. The existing test asserted the row count was zero afterwards, locking
  the bug in.

  The lock table is gone entirely. It was a hand-rolled stand-in for `BEGIN
  IMMEDIATE`, which is how SQLite itself takes the write lock before the
  `SELECT` that picks a job — a plain `BEGIN` defers it until the first write
  and leaves the read unprotected, which is the race the table existed to
  close. `initialize_database` drops `sqlt_loco_queue_lock` if it is still
  there, so upgrading needs no action. The mutual-exclusion guarantee is now
  covered by a test that runs concurrent dequeues and fails without the fix.

- **`examples/reference_spa` did not build for anyone but its author.** Its
  manifest pinned `loco-rs` to an absolute path on the maintainer's machine
  (`path = "/Users/…/loco"`) — the shape `LOCO_DEV_MODE_PATH` generates — so
  every other checkout failed with `failed to load manifest for dependency
  loco-rs`. It is `path = "../.."` now, and CI runs both examples' test suites,
  which is what caught it.

- **snipdoc overwrote the translated READMEs.** Each translation carries the
  same `<snip>` regions as `README.md`, so every injection run replaced the
  translated tagline, install comment and `loco new` transcript with the English
  source — and CI's `snipdoc check` then failed whenever a translator put their
  version back. The Spanish and Vietnamese READMEs had already lost theirs.
  `snipdoc-config.yml` now excludes the translated filenames from the walk, so
  only the canonical `README.md` is injected, and the two translations are
  restored to what their translators wrote.

- **On the Redis queue driver, a job vanished from every operator tool the
  moment it stopped being runnable.** `get_jobs` enumerated jobs by walking the
  queue and processing keys, but `complete_job` and `fail_job` both remove the
  id from the processing set and add it to no queue — so `jobs dump --status
  failed`, `jobs purge`, `clear_by_status` and `clear_jobs_older_than` all
  silently reported nothing for completed and failed jobs. It now enumerates the
  `job:*` keys, which are the record of a job's existence, and consults the
  processing sets only to tell a job a worker is holding from one still queued.
  The existing test could not catch this: it asserted inside
  `for job in &failed_jobs`, which passes on an empty list — and the list was
  always empty.
- **Test snapshots no longer break west of UTC.** `get_cleanup_date`'s
  timestamp rule ended in `\+\d{2}:\d{2}`, matching only a *positive* UTC
  offset. It is the only rule that consumes the offset, so on a machine west of
  UTC the timestamp fell through to the offset-less rules, which stop at the
  seconds — redacting to `DATE-03:00` instead of `DATE` and failing every
  snapshot carrying a timestamp. A freshly generated app therefore had a red
  test suite out of the box for everyone west of UTC, and had since 0.14.
  CI never caught it because GitHub runners are UTC.
  ([#1802](https://github.com/loco-rs/loco/pull/1802))
- **Every npm advisory in `website/` and the reference SPA is cleared**, and the
  `loco new` frontend template no longer pins `vite`/`@vitejs/plugin-react` to
  a floating `"latest"`. Its `react-router` floor moves to `^8.3.0`, below which
  a generated app resolves a version with an RSC-mode CSRF bypass.
- **Config templating is now YAML-safe** (`<%= ... %>` instead of `{{ ... }}`).
  `{` is a YAML flow-mapping indicator, so `port: {{ get_env(...) }}` was never
  valid YAML at rest — it only parsed because Loco's template pass rewrote the
  file first. Any tool reading the file *as YAML* (prettier, yaml-language-server,
  format-on-save) restructured it into `{ { ... } }` and broke startup.
  `<` is not a YAML indicator, so the new form is an ordinary string scalar:
  config files are valid YAML before rendering and survive formatting untouched.
  Three tag forms are supported — `<%= expr %>`, `<% stmt %>`, `<%# text %>`.
  **Not a breaking change:** legacy `{{ }}` still renders, with a deprecation
  warning. ([#1727](https://github.com/loco-rs/loco/issues/1727))
- **Environment variables in generated configs are no longer baked in at scaffold
  time.** Because the generator and the runtime shared the same `{{ }}` delimiters,
  several lookups in a newly generated app were evaluated by `loco new` and frozen
  into the file — so `PORT`, `BINDING`, `LOG_LEVEL`, `DB_LOGGING` and `MAILER_HOST`
  silently had no effect at runtime. Only the handful of lookups that were manually
  `{% raw %}`-escaped survived. The two layers now use distinct delimiters, so every
  lookup reaches runtime as intended (and the `{% raw %}` escaping is gone from the
  templates).

- **Generated apps pinned a `loco-rs` that could not read their own config.**
  `LOCO_VERSION` — the version requirement written into every generated
  `Cargo.toml` — still said `1.0`, so a fresh app resolved the newest published
  1.0.x, which renders the new `<%= ... %>` config delimiters literally and then
  fails to parse them: the app compiled and died at boot. The floor now tracks
  the release, enforced by a test.

  The reason it went stale is its own bug: `cargo xtask bump` maintains that
  constant, but its search pattern was pinned to the literal `"0.13"` and a
  pattern that matched nothing was a printed note rather than an error. Every
  release since 0.14 reported success while leaving the floor untouched. A
  no-match is now a failure, and each of the four version sites the tool
  rewrites is covered by a test that it still matches.

- **`config/production.yaml` was generated as a 0-byte file** — copied verbatim
  instead of rendered, since the CLI generator rewrite — and the generated
  `.gitignore` excluded it, so even a correct one would never have reached a
  server. Production is now a real template: backtraces off, `json` logs,
  `0.0.0.0` binding rather than loopback (unreachable from outside a container),
  and a connection pool that isn't the development default of one. The ignore
  rule is gone; secrets live in the environment, so the file is infrastructure
  and belongs in the repository. `local.yaml` stays ignored.

- **`doctor --production` checked the wrong environment and ran fewer checks.**
  The flag never selected an environment — it filtered checks while the config
  under test stayed whatever was ambient, and the default environment is
  `development`. On a server without `LOCO_ENV` set it reported a clean bill of
  health for the development database and never opened the production config.
  It is now a deprecated alias for `--environment production`, and production
  additionally checks settings that are harmless in development and not live: a
  loopback binding, `dangerously_truncate`/`dangerously_recreate`, a queue that
  flushes on startup, backtraces left on.

- **A generated migration that could not be registered reported success.**
  Through rrgen 0.5, a `before:` injection whose anchor line was absent was not
  an error — it rewrote the file unchanged and still printed `injected: …`. A
  `migration/src/lib.rs` without the `inject-above` comment therefore accepted
  the `mod` declaration and silently dropped the `Box::new(..)` registration:
  the migration compiled, never ran, so the table was never created, `db
  entities` correctly wrote nothing, and the first insert 500'd at runtime.

  Fixed at the source in **rrgen 0.6**, which Loco now requires: an injection
  that cannot find its anchor fails, naming the file, the pattern and the
  content it could not place, and the failed generation writes nothing at all —
  so restoring the anchor and re-running does the whole job. This covers every
  injecting generator, not just migrations: controllers, scaffolds, tasks and
  the frontend route table all inject the same way.

  Loco additionally fails generation if any migration in `migration/src/` is
  unregistered. That catches what an injection cannot see: a registration that
  went missing on an earlier run or by hand.

- **Generated apps are now booted, not just compiled, by the test suite.** The
  wizard matrix starts each generated app and requires `/_ping` and `/_health`
  to answer 200, in `development` and again in `production` — the environment
  nothing exercised. The three fixes above all shipped in states that a full
  green suite could not see, because nothing in the repository ever ran the
  artifact a user receives.

- **`loco-gen` no longer depends on Tera.** The dependency was unused; `rrgen`
  carries its own Tera 1, which coexists with Tera 2 without API contact.
- **`loco new --assets serverside --embedded-assets` produced an app that did
  not compile.** The embedded view engine was missing the
  `build_with_post_process` constructor that the generated view-engine
  initializer calls, so the two engines were not interchangeable. Both now
  expose the same constructors, and the wizard test matrix builds this
  combination end to end instead of only asserting on wizard settings.
- **Generated server-side apps now test their own view rendering.** A new
  `tests/views/` case renders the shipped Tera template through the real view
  engine, including the i18n `t()` function, so view-engine regressions surface
  in an app's own test suite rather than at boot.
- **Scaffolding a second resource broke the SPA build.** Every resource's pages
  are named `List`/`New`/`Show`/`Edit`, and the route injection imported them
  bare — so the second `generate scaffold` in an app injected a duplicate
  binding for all four names into `frontend/src/routes.tsx`. Imports are now
  aliased per resource (`List as PostsList`). Existing `routes.tsx` entries are
  your code and are untouched; new scaffolds emit the aliased form.
- **A custom foreign-key column name never reached the scaffold.**
  `user:references:admin_id` names the FK column explicitly and the migration
  honours it, but the DTO and controller derived `user_id` regardless and
  referenced a column the entity does not have.
- **`code:string!^` failed to parse.** Only one of the two flag suffixes was
  stripped, leaving `string!` as the type name and reporting an unknown base
  type the user never wrote. Both flags now parse in any order and in either
  position (`decimal_len!^:8:24` and `decimal_len:8:24^!` are the same column).
  `^` already implies non-null, so the combination is redundant — but it should
  not have been an error.
- **`Pager` could not deserialize its own output.** A serialize-only rename
  emitted `{"results": .., "pagination": ..}` while the deserializer looked for
  `info`, so the derived `Deserialize` — public API — failed with
  `missing field \`info\``. No wire format changed.
- **The legacy config-delimiter deprecation warning was unreachable.** It was a
  `tracing::warn!` emitted during `load_config`, which runs before
  `logger::init`, so no subscriber existed to receive it. It now goes to stderr,
  where config-time diagnostics belong.
- **The generated config shows how to move the JWT out of the header.**
  `auth.jwt.location` (cookie or query parameter) had no example in a generated
  app.
- **`auth.jwt.location` reports what is actually wrong with it.** The setting
  was an `#[serde(untagged)]` enum, so a misspelled `from: cookie`, a `Cookie`
  with no `name`, and a bare scalar all produced the identical `data did not
  match any variant of untagged enum JWTLocationConfig`. The inner error now
  propagates — `unknown variant \`cookie\`, expected one of \`Bearer\`,
  \`Query\`, \`Cookie\``. The accepted YAML and the serialized form are
  unchanged.
- **`generate migration Rename<Old>To<New>On<Table>` now generates a real
  migration** instead of a `todo!()` stub announced as ready to run. Adds
  `loco_rs::schema::rename_column`. When a name genuinely can't be inferred the
  stub remains — silently succeeding would record it as applied — but the
  generator now says it is unimplemented, that `db migrate` will panic, and
  which names it does understand.
- **The starter's tests no longer snapshot whole models.** Adding one column to
  `users` broke five generated tests at once, because they pinned an entire
  `users::Model` `Debug` dump. They snapshot the fields under test now, and
  assert the rest directly.
- **The generated `development.yaml` names a mail catcher and the `PORT`
  override.** The dev mailer targets `localhost:1025` with no `stub`, so mail
  failed unless something was listening and nothing said what; and every app
  defaults to port 5150, so a second one collides.
- **A missing database is documented as the one-way door it is.** `--db none`
  turns off `with-db`, and no generator reverses it; there is now a how-to with
  the exact procedure, and the clientside React/`ts-rs` mode — previously
  undocumented in full — has a guide of its own.


## 1.0.1 - 2026-07-31

A documentation and CLI-ergonomics patch. No behavior changes to the framework
runtime.

### Fixed

- **`generate scaffold`/`controller` no longer error on the old `--api` flag.**
  The 1.0 adaptive-generator rebuild removed the `--api`/`--html`/`--htmx`
  (and `-k/--kind`) flags — scaffold now auto-detects headless vs. clientside
  from `frontend/`, and controllers are always JSON API controllers — but the
  docs (including the *Your first app* tutorial) still showed `--api`, so
  copy-pasting them failed with `error: unexpected argument '--api' found`. The
  generators now accept the old flags for compatibility: `--api` is a no-op
  (it's the headless default) and `--html`/`--htmx` return a clear message
  pointing at the React SPA frontend that replaced server-rendered views.
  ([#1790](https://github.com/loco-rs/loco/issues/1790))
- **Docs resync.** Removed every reference to the removed scaffold/controller
  kind flags and the deleted `ScaffoldKind` enum / `mappings.json` across the
  tutorials, how-to guides, and CLI/generators reference; corrected the
  field-type reference to describe `loco-gen/src/column.rs` (including that
  `array:int` is now a 64-bit `BigInt` array, consistent with the scalar
  `int` → `i64` change).


## 1.0.0 - 2026-07-25

1.0.0 is the first stable Loco release — a single, intentionally-breaking
milestone. Its headline is the move to **Sea-ORM 2.0**, alongside first-class
LLM/agent support, priority queues, a broad dependency modernization, and a deep
hardening pass across the queue, storage, config, error, remote-IP, and
middleware subsystems. Follow the step-by-step
[0.16 → 1.0 upgrade guide](https://loco.rs/docs/extras/upgrades/).

### Breaking Changes

- **Sea-ORM 2.0 + sqlx 0.9.** Bump `sea-orm`/`sea-orm-migration` to **`2.0`**
  (app + `migration` crate), direct `sqlx` to `0.9`, update the Sea-ORM CLI, and
  regenerate entities. Raw-`Statement` calls gain a `_raw` suffix; runtime SQL
  strings need `AssertSqlSafe`. MSRV is **1.94** (sea-orm 2.0.0 declares it).
  (Adopted from the SeaQL fork and
  [#1698](https://github.com/loco-rs/loco/pull/1698).)
- **Generated primary/foreign keys are now 64-bit (BIGINT / `i64`).** Also the
  `int`/`unsigned` field types generate 64-bit columns. Only affects newly
  generated code; existing tables are untouched.
- **Priority queues — Redis backend change.** The Redis worker moved from Lists
  to Sorted Sets (ZSET) to support priority; **drain existing Redis queues before
  upgrading**. Postgres/SQLite auto-migrate a `priority` column (no action).
  ([#1693](https://github.com/loco-rs/loco/pull/1693))
- **`Worker::perform_later()` returns the job ID** (`Result<String>`), and
  `Queue::enqueue()` returns `Result<Option<String>>`. Existing
  `perform_later(...).await?;` keeps working. ([#1624](https://github.com/loco-rs/loco/pull/1624), fixes [#1623](https://github.com/loco-rs/loco/issues/1623))
- **`PageResponse<T>` exposes `meta: PagerMeta`** instead of flat
  `total_pages`/`total_items` (also carries `page`/`page_size`). ([#1685](https://github.com/loco-rs/loco/pull/1685), fixes [#1683](https://github.com/loco-rs/loco/issues/1683))
- **View engine:** use `engines::TeraView::build_with_post_process(...)` instead
  of `TeraView::build()?.post_process(...)` in `after_routes`.
- **Dependency majors:** `thiserror` 1→2, `tower` 0.4→0.5, `heck`→0.5,
  `byte-unit` 4→5, `ipnetwork` 0.20→0.21, `strum`→0.27, `redis` 0.31→1,
  `bb8-redis`→0.26, `opendal` 0.54→0.57; `serde_yaml`→`serde_yaml_ng`.
  Transitive for most apps.

  *(Correction, made while preparing 1.1.0: this entry previously read
  `opendal` 0.54→0.58.1 and credited this release with escaping
  RUSTSEC-2026-0194/0195. That was wrong — the note was edited into this
  section after the fact, while 1.0.0 and 1.0.1 both shipped `opendal = "0.57"`.
  The advisories are addressed in 1.1.0; see its Security section.)*
- Removed the dead `loco-cli` crate (superseded by `loco-new`, the published
  `loco` binary).
- **`ExtraDbInitializer` removed; use `MultiDbInitializer`.** The
  single-extra-connection initializer (`initializers.extra_db`, which layered a
  bare `Extension<DatabaseConnection>`) is gone. Use `MultiDbInitializer` with a
  one-entry `initializers.multi_db` map instead, and extract the connection with
  `Extension<MultiDb>` + `multi_db.get("<name>")`. This collapses two
  near-identical initializers into one named-connections abstraction.
- **`AppContext` is now `#[non_exhaustive]`.** Construct it with
  `AppContext::builder(environment, db, config)` (or `builder(environment,
  config)` without the `with-db` feature) followed by optional
  `.queue_provider(..)`/`.mailer(..)`/`.storage(..)`/`.cache(..)`/`.shared_store(..)`
  and `.build()`. Direct struct-literal construction and exhaustive pattern
  matches on `AppContext` from outside the crate no longer compile; field
  access (`ctx.db`, `ctx.config`, `State`/`FromRef` extraction) is unchanged.
  This makes future context fields non-breaking to add.
- **Storage `MirrorStrategy` and `BackupStrategy` merged into `ReplicatedStrategy`.**
  The two strategies were the same primary-plus-secondaries replication engine;
  they are now one `storage::strategies::replicated::ReplicatedStrategy` with a
  single `FailurePolicy` enum. Migrate: `MirrorStrategy::new(p, s, MirrorAll)` →
  `ReplicatedStrategy::mirror(p, s, FailurePolicy::FailIfAny)`;
  `BackupStrategy::new(p, s, BackupAll)` → `ReplicatedStrategy::backup(p, s,
  FailurePolicy::FailIfAny)`. Old `FailureMode` maps: `AllowMirrorFailure`/
  `AllowBackupFailure` → `AllowAll`, `AtLeastOneFailure` → `AllowSingleFailure`,
  `CountFailure(n)` → `FailAtFailures(n)`. Secondary writes for the former
  backup strategy now run concurrently (previously sequential); the collected
  errors and failure decision are unchanged.
- **Local storage driver no longer defaults its root to `/`.**
  `storage::drivers::local::new()` previously rooted the filesystem store at
  `/`, so any key — including one derived from user input — resolved against the
  whole disk (e.g. downloading key `etc/passwd` read `/etc/passwd`). It now roots
  at the current working directory. Apps that relied on absolute-path keys should
  switch to `local::new_with_prefix("/your/root")` to opt back into an explicit
  absolute root.
- **Background queue reworked into a `QueueProvider` adapter interface.** The
  `bgworker::Queue` enum (`Postgres`/`Sqlite`/`Redis`/`None`) is now a newtype
  over `Arc<dyn QueueProvider>`, so backends are pluggable (implement
  `QueueProvider` and wrap with `Queue::from_provider`). All existing methods
  (`enqueue`, `register`, `run`, `ping`, `cancel_jobs`, `clear_by_status`,
  `requeue`, …) keep the same signatures and behavior. Only two source-level
  changes affect callers: construct a no-op queue with `Queue::empty()` instead
  of `Queue::None`, and code that pattern-matched the enum variants (e.g.
  `Queue::Postgres(pool, ..)` to reach the raw pool) no longer compiles — use the
  provider methods instead.
- **Fallback middleware defaults to `404`.** When the built-in fallback is
  enabled without an explicit `code`, it now returns `404 Not Found` (matching
  its docs and the bundled not-found page) instead of `200 OK`. Apps that relied
  on the enabled fallback returning `200` must set `code: 200` explicitly. The
  file-based fallback is unaffected (`ServeFile` reports its own status).
- **`{env}.local.yaml` now deep-merges over `{env}.yaml`.** Previously the first
  existing file won and the other was ignored, so a `.local.yaml` had to restate
  the whole config. Both files are now layered with local precedence: mappings
  merge recursively; scalars and sequences in local replace the base value
  (sequences are not concatenated). Base keys now persist unless explicitly
  overridden.
- **More accurate HTTP status codes for errors.** `IntoResponse for Error`
  previously collapsed ~28 of 35 variants to `500`. `Model(EntityNotFound)` now
  returns `404`, `Model(EntityAlreadyExists)` returns `409`, model validation
  and form-body rejections return `4xx` (matching JSON rejections) instead of
  `500`. Genuinely-internal errors still return a generic `500`. Handlers that
  asserted on the old `500`s will observe the corrected codes.
- **`JWT::algorithm()` restricted to the HMAC family.** It now takes a new
  `loco_rs::auth::jwt::JWTAlgorithm` enum (`HS256`/`HS384`/`HS512`) instead of
  `jsonwebtoken::Algorithm`. Asymmetric algorithms — which could never work with
  Loco's shared base64 secret and silently produced broken tokens — are no longer
  representable.
- **`remote_ip` middleware rebuilt on `axum-client-ip`; `trusted_proxies`
  removed.** Previously this middleware walked `X-Forwarded-For` right-to-left,
  skipping any address in a configurable `trusted_proxies` CIDR list (or a
  built-in RFC-1918 + loopback list), so it could see through a chain of one or
  more trusted proxies. It now trusts exactly **one** configured source
  (`source: ClientIpSource`, default `RightmostXForwardedFor`) and does **no**
  CIDR filtering — for the default it takes the last comma-separated value of the
  last `X-Forwarded-For` header verbatim, private or not. Single reverse-proxy
  deployments are unaffected. **Multi-hop topologies (CDN → LB → ingress) must
  now configure their innermost hop to set the client IP (e.g. nginx
  `set_real_ip_from`/`real_ip_recursive`), or point `source` at a provider header
  (`CfConnectingIp`, `CloudFrontViewerAddress`, `XRealIp`, `ConnectInfo`, …).**
  Note: an old config's `trusted_proxies:` key is silently ignored (unknown
  field), so review `remote_ip` before upgrading — this is a silent
  security-relevant behavior change, not a load error. The `RemoteIP` extractor
  and its `Display` output are unchanged.
- **`auth_jwt` feature renamed to `auth`.** Update `features = ["auth_jwt"]` → `["auth"]`
  (it gates JWT auth and the `ApiToken` extractor, as before).
- **Background-queue features collapsed.** `bg_pg`/`bg_sqlt` → `worker`
  (Postgres+SQLite; free once `sqlx` is compiled), `bg_redis` → `worker_redis`
  (adds `dep:redis`). `default` now has `worker` (not Redis). A Redis queue needs
  `worker_redis`; the queue backend is selected at runtime by `queue.kind`.
- **The `integration_test` feature is removed.** It had no `#[cfg]` reference
  anywhere in the tree, so enabling it never did anything — but it was a
  declared feature of released 0.16.x, so `features = ["integration_test"]` in
  a dependent's `Cargo.toml` now fails to resolve. Delete the entry.
- **Mailer `Template::new(dir)` now returns `Result`** (call `Template::new(dir)?`).
  Email templates render through a full Tera instance so they support inheritance
  and shared templates. Standard usage via `Mailer::mail_template` is unchanged.
  ([#1694](https://github.com/loco-rs/loco/pull/1694))
- **`Vars::cli_arg` returns `Result<&str>`** (was `Result<&String>`). Callers that
  relied on `&String` (e.g. `.clone()` into a `String`) should use `.to_owned()`.
  ([#1732](https://github.com/loco-rs/loco/pull/1732))

### Added

- **First-class LLM / agent support.** Root `AGENTS.md` teaches agents to build
  Loco apps; `llms.txt` / `llms-full.txt` are served from the site
  (llmstxt.org). Every `loco new` app ships an app-level `AGENTS.md`.
- **Priority queues** with `Worker::perform_later_with_priority(...)`; mailer
  jobs default to priority `100`. ([#1693](https://github.com/loco-rs/loco/pull/1693))
- **Mailer implicit TLS (SMTPS / port 465)** via `mailer.smtp.tls`. ([#1774](https://github.com/loco-rs/loco/pull/1774), fixes [#1773](https://github.com/loco-rs/loco/issues/1773))
- **Run the scheduler without a worker** — `--scheduler` flag +
  `StartMode::ServerAndScheduler`/`WorkerAndScheduler`. ([#1742](https://github.com/loco-rs/loco/pull/1742), fixes [#1737](https://github.com/loco-rs/loco/issues/1737))
- Email headers support in the mailer ([#1700](https://github.com/loco-rs/loco/pull/1700)).
- **Multi-recipient emails.** `Mailer::mail_multi` / `mail_template_multi` and the
  `MultiEmail` / `MultiArgs` types send one email to multiple To/CC/BCC
  recipients (processed by a dedicated `MultiMailerWorker`). ([#1764](https://github.com/loco-rs/loco/pull/1764))
- **Email template inheritance & shared templates.** Mailer templates support
  Tera `{% extends %}` / `{% block %}` and can share a common layout via
  `Mailer::mail_template_with_shared` / `Template::new_with_shared`. `loco generate
  mailer` now scaffolds a `src/mailers/shared/` base layout that the welcome
  template extends. ([#1694](https://github.com/loco-rs/loco/pull/1694))
- "Create user" task ([#1670](https://github.com/loco-rs/loco/pull/1670)).
- `UuidUniqWithDefault` and `UuidWithDefault` types ([#1642](https://github.com/loco-rs/loco/pull/1642)).
- Allow overriding a secure header ([#1659](https://github.com/loco-rs/loco/pull/1659)).
- **`Mailer::deliver_now` / `mail_template_now` for synchronous sends.**
  Complements `Mailer::mail`/`mail_template` (which enqueue via the background
  worker, like Rails `deliver_later`) with an inline send that bypasses the
  queue (Rails `deliver_now`).
- **`MiddlewareStackExt` for surgical middleware-stack edits.** Inside
  `Hooks::middlewares`, tweak the default stack instead of rebuilding it:
  `stack.insert_before("cors", ..)`, `.insert_after(..)`, `.replace(..)`, and
  `.delete("logger")` — matched by middleware name (Rails'
  `config.middleware.insert_before`/`delete`). Available via the prelude.
- **Optional JWT extraction.** `JWT` now implements `OptionalFromRequestParts`,
  so a handler can take `Option<JWT>` to serve authenticated and anonymous
  callers from one endpoint (`Some` when a valid token is present, `None`
  otherwise).
- **Ergonomic verb-explicit route methods.** `Routes` now has `get`/`post`/
  `put`/`delete`/`patch`/`head`/`options`/`trace` builder methods —
  `Routes::new().get("/ping", ping)` alongside the existing
  `.add("/ping", get(ping))`. They record the HTTP verb directly (exact
  `cargo loco routes` output without relying on the debug-format regex).
  Purely additive; `add` is unchanged.
- **Opt-in background-job reaper (visibility timeout).** Each queue backend's
  config accepts a `reaper: { age_minutes, interval_seconds }` block. When set,
  the worker periodically requeues jobs stranded in `Processing` (e.g. by a
  crashed worker) back to `Queued`, instead of requiring a manual
  `cargo loco jobs requeue`. Disabled by default — existing behavior is unchanged.
- **TLS to managed Postgres and Redis.** Postgres TLS works via the connection
  URL (`sslmode=require`, `sslrootcert=...`) with no feature flag, and a new
  `redis_tls` feature enables `rediss://` for both the queue and cache Redis
  backends (webpki roots, pure-Rust `ring` provider — no C toolchain). New
  how-to: "Connect to Postgres and Redis over TLS". ([#1191](https://github.com/loco-rs/loco/issues/1191), [#1341](https://github.com/loco-rs/loco/issues/1341))
- **Typed, streaming `db::dump::<A>()`** — counterpart to `db::seed::<A>()` that
  streams rows through their entity `Model` straight to disk (memory bounded to
  a single row) with full type fidelity. New `Hooks::dump` (default dumps every
  table; override it to call `db::dump` per entity) backs `cargo loco db seed
  --dump`. ([#1691](https://github.com/loco-rs/loco/issues/1691))
- **`logger::init_layer` / `logger::init_env_filter` are now public** building
  blocks, so an app overriding `Hooks::init_logger` can reuse Loco's formatting
  and filter policy while adding its own layers (e.g. `tracing-flame`, OTLP).
  ([#1753](https://github.com/loco-rs/loco/issues/1753))

### Changed

- Wrap `TeraView` in `Arc` to reduce runtime memory usage ([#1703](https://github.com/loco-rs/loco/pull/1703)).
- Refactor users model to reuse `find_by_api_key` in `Authenticable` ([#1706](https://github.com/loco-rs/loco/pull/1706)).
- Split error detail generic parameters ([#1709](https://github.com/loco-rs/loco/pull/1709)).
- Update `loco-new` for the new Rhai version ([#1704](https://github.com/loco-rs/loco/pull/1704)).
- Replaced hand-rolled `Cargo.lock` parsing with the `cargo-lock` crate; retired
  `duct_sh`.
- **`Error` → HTTP-status mapping is now exhaustive.** The `IntoResponse for
  Error` match dropped its trailing `_ => 500` wildcard: every variant (and every
  nested `ModelError` variant) is now classified explicitly, so adding a new
  error variant is a compile error until its status is chosen — it can no longer
  silently default to `500`. The `Error` enum is also reorganized into
  client-facing vs internal/infra regions. Behavior is unchanged (all infra
  errors still map to `500`); no variant was renamed, so existing code is
  unaffected.
- **Rust edition 2024.** `loco-rs`, `loco-gen`, `xtask`, and the `loco` new-app
  generator now compile on edition 2024 (MSRV floor unchanged at 1.94; edition
  2024 needs ≥ 1.85). Editions are per-crate, so apps depending on Loco need not
  change. Newly generated apps stay on edition 2021 for now.
- Deduplicated the Postgres and SQLite background-queue providers: the shared
  `Job`/`JobRegistry`/`RunOpts` now live in one module behind a `Driver` trait
  (internal refactor, no behavior or API-path change).
- In-memory cache now uses `moka::future::Cache` instead of wrapping the
  synchronous cache behind `#[async_trait]` (removes a sync-behind-async smell;
  no API change).
- Cookie token extraction now uses `axum_extra`'s `Cookie::value()` instead of
  hand-parsing the cookie string (byte-identical behavior).
- Internal de-duplication pass (no public-API-path or behavior change unless
  noted): the response helpers in `format` are now single-sourced through
  `RenderBuilder`; the `JWT`/`JWTWithUser` extractors share one validate/decode
  helper; the six validate extractors are generated from shared decoder fns +
  two error-tier macros; the byte-identical `Job` struct is shared across the
  SQL and Redis queue backends; the twin `cli::main` functions share one
  `dispatch_common`; and duplicate env-var name constants were removed.
- `format`'s two response paths were converged onto axum's canonical behavior:
  `RenderBuilder::json` and `RenderBuilder::redirect_with_header_key` are now
  infallible with respect to bad input (they return a `500` response, matching
  `axum::Json` / `axum::response::Redirect`) instead of returning `Err`.

### Fixed

- **`db seed --dump` datetime round-trip on SQLite.** Loco's timestamptz columns
  default to `CURRENT_TIMESTAMP`, which SQLite stores as space-separated text
  (`"YYYY-MM-DD HH:MM:SS"`); dumps captured that verbatim and then failed
  chrono's RFC3339 parse on re-seed (`Json("premature end of input")`). Dumps now
  normalize such datetimes to RFC3339 (already-RFC3339 text is untouched).
  ([#1736](https://github.com/loco-rs/loco/issues/1736), [#1691](https://github.com/loco-rs/loco/issues/1691))
- `cargo fmt` error in `loco-new` ([#1669](https://github.com/loco-rs/loco/pull/1669)).
- UUID pattern in form field generation ([#1665](https://github.com/loco-rs/loco/pull/1665)).
- Clippy warnings for recent Rust ([#1705](https://github.com/loco-rs/loco/pull/1705)).
- Add tests for the auth extractor ([#1671](https://github.com/loco-rs/loco/pull/1671)).
- **Postgres/SQLite queue backends now behave consistently.** Two divergences
  between the Postgres and SQLite job backends are fixed: (1) `enqueue` on
  Postgres previously *swallowed* a tag-serialization error (storing `tags =
  null`); it now propagates the error like SQLite. (2) `complete_job` without a
  repeat interval on Postgres stamped `run_at = NOW()` on the completed row while
  SQLite left it untouched; Postgres now leaves `run_at` as-is, matching SQLite
  (the interval path still reschedules `run_at` on both). The shared `to_job` row
  mapper is now single-sourced across both backends.
- **Storage mirror fan-out no longer stops at the first failing secondary.**
  `rename`, `copy`, and `upload_stream` checked the failure mode *inside* the
  secondary loop and returned early on the first failure, silently leaving later
  mirrors stale (`upload`/`delete` were already correct). All five mutating
  methods now share one helper that attempts every secondary (concurrently) and
  applies the failure mode once.
- **Postgres `BOOLEAN` columns are no longer dropped from `dump_tables`.** The
  decode probe chain had no `bool` arm, so PG booleans (which don't fall back to
  the numeric arms like SQLite's integer-backed booleans) were silently omitted.
- **`Hooks::on_shutdown` now runs in worker-only start modes.** `WorkerOnly` and
  `WorkerAndScheduler` bypassed `H::serve` (the hook's only caller); the shutdown
  hook is now invoked on their shutdown path too.
- **Postgres admin/maintenance URI is derived with the `url` crate.** Building it
  via `db_uri.replace(db_name, "/postgres")` corrupted the URI when the database
  name also appeared in the host or credentials.
- **Foreign-key names are normalized consistently.** `reference_id` received a
  normalized table name in `create_table` but raw names in
  `add_reference`/`remove_reference`, so irregular plurals produced mismatched FK
  column/constraint names between creation and later add/remove.
- `ViewEngine` extractor now rejects gracefully (HTTP `500`) when the opt-in Tera
  layer is absent, instead of declaring `Infallible` and then panicking.
- `cargo loco routes` lists every HTTP verb of a multi-method route (route
  introspection previously reported only the first).
- Removed a fossilized 3-second `sleep` on every Redis queue boot (a leaked
  test-isolation artifact; Postgres/SQLite had no equivalent).
- Password redaction in test snapshots (`cleanup_user_model`) now targets the
  quoted value precisely; the previous pattern had a degenerate quantifier that
  swallowed the field following `password`.
- Postgres test-database cleanup now completes synchronously (a joined worker
  thread) instead of a fire-and-forget task, so parallel test runs no longer
  leak databases; `PostgresTest` also builds its connection strings with the
  `url` crate rather than a corruption-prone substring replace.
- `RenderBuilder::template` now threads the builder's chained `status`/`header`/
  `etag`/`cookies` through to the response; it previously delegated to the free
  `html()` and silently dropped them.
- `llms.txt`: two `Core concepts` links pointed at doc pages that don't exist
  (`the-app/configuration/`, `the-app/testing/`); repointed to the sections that
  actually document them. A new `cargo xtask llms-check` CI step now verifies the
  curated LLM docs against the docs tree so these links can't drift silently.

### Removed

- **`Error` enum narrowing.** Removed four low-value/dependency-leaking variants
  that all mapped to HTTP 500 and were never matched: `Error::EnvVar`,
  `Error::SemVer`, `Error::TaskJoinError`, and `Error::Hash` (hashing errors now
  surface as `Error::Message`). `Error` remains `#[non_exhaustive]`, so exhaustive
  matches already require a wildcard arm and are unaffected.
- Deleted shipped-but-dead code: the never-compiled
  `controller/middleware/_archive/content_etag.rs` module and a commented-out
  block of backtrace-blocklist regexes.

## v0.16.4 
- Feat: decouple JWT authentication from database dependency. [https://github.com/loco-rs/loco/pull/1546](https://github.com/loco-rs/loco/pull/1546)
- Fix: add sqlx dependency to with-db feature. [https://github.com/loco-rs/loco/pull/1557](https://github.com/loco-rs/loco/pull/1557)
- Remove the deprecated `--link` generate command and fix the table name creation. [https://github.com/loco-rs/loco/pull/1556](https://github.com/loco-rs/loco/pull/1556)
- Support underscore for migration join table. [https://github.com/loco-rs/loco/pull/1562](https://github.com/loco-rs/loco/pull/1562)
- Fix: resolve deployment CLI argument parsing issue. [https://github.com/loco-rs/loco/pull/1566](https://github.com/loco-rs/loco/pull/1566)
- Add database enum support (Postgres only). [https://github.com/loco-rs/loco/pull/1593](https://github.com/loco-rs/loco/pull/1568)
- Remove duplicated #[async_trait::async_trait]. [https://github.com/loco-rs/loco/pull/1593](https://github.com/loco-rs/loco/pull/1572)
- Clippy fixes for Rust 1.89. [https://github.com/loco-rs/loco/pull/1593](https://github.com/loco-rs/loco/pull/1593)
- Improvement: do not hot-reload unless files have changed. [https://github.com/loco-rs/loco/pull/1552](https://github.com/loco-rs/loco/pull/1552)
- Feat: add --without-tz flag for controlling timestamp generation. [https://github.com/loco-rs/loco/pull/1592](https://github.com/loco-rs/loco/pull/1592)
- Support extra fields when generating the join table migration. [https://github.com/loco-rs/loco/pull/1595](https://github.com/loco-rs/loco/pull/1595)
- Convert validator to trait-based API (add ValidatorTrait, keep derive adapter, update docs). [https://github.com/loco-rs/loco/pull/1597](https://github.com/loco-rs/loco/pull/1597)
- Rename dockerfile to Dockerfile. [https://github.com/loco-rs/loco/pull/1574](https://github.com/loco-rs/loco/pull/1574)
- Enable edit CORS expose headers. [https://github.com/loco-rs/loco/pull/1599](https://github.com/loco-rs/loco/pull/1599)
- Adding new imports about multipart. [https://github.com/loco-rs/loco/pull/1600](https://github.com/loco-rs/loco/pull/1600)
- Adding readiness default endpoint. [https://github.com/loco-rs/loco/pull/1563](https://github.com/loco-rs/loco/pull/1563)
- Add Route methods to make collecting and nesting easier. [https://github.com/loco-rs/loco/pull/1608](https://github.com/loco-rs/loco/pull/1608)
- Add streaming support for both download and upload. [https://github.com/loco-rs/loco/pull/1610](https://github.com/loco-rs/loco/pull/1610)
- Fix Clippy for Rust 1.90. [https://github.com/loco-rs/loco/pull/1630](https://github.com/loco-rs/loco/pull/1630)
- Loco CLI: Update rhai version. [https://github.com/loco-rs/loco/pull/1631](https://github.com/loco-rs/loco/pull/1631)


## v0.16.3
- Support nullable foreign keys with `references?` syntax. [https://github.com/loco-rs/loco/pull/1544](https://github.com/loco-rs/loco/pull/1544)
- **HOTFIX**: **Breaking changes** Fixed a critical issue introduced in version `v0.16.2` that caused `cargo build --release` to fail after merging #1540. [https://github.com/loco-rs/loco/pull/1551](https://github.com/loco-rs/loco/pull/1551)
- Add an API to re-send verification mail. [https://github.com/loco-rs/loco/pull/1456](https://github.com/loco-rs/loco/pull/1456)
- Adding to ci cargo build --release. [https://github.com/loco-rs/loco/pull/1553](https://github.com/loco-rs/loco/pull/1553)

### Breaking Changes

In file `src/initializers/view_engine.rs`, modify the method `after_routes`:

Before

```rust
async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
	#[allow(unused_mut)]
	let mut tera_engine = engines::TeraView::build()?;
	if std::path::Path::new(I18N_DIR).exists() {
		let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
			.shared_resources(Some(&[I18N_SHARED.into()]))
			.customize(|bundle| bundle.set_use_isolating(false))
			.build()
			.map_err(|e| Error::string(&e.to_string()))?;
		#[cfg(debug_assertions)]
		tera_engine
			.tera
			.lock()
			.expect("lock")
			.register_function("t", FluentLoader::new(arc));

		#[cfg(not(debug_assertions))]
		tera_engine
			.tera
			.register_function("t", FluentLoader::new(arc));
		info!("locales loaded");
	}

	Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
}
```

After (use `post_process` to add i18n initialization code)

```rust
async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
	let tera_engine = if std::path::Path::new(I18N_DIR).exists() {
		let arc = std::sync::Arc::new(
			ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
				.shared_resources(Some(&[I18N_SHARED.into()]))
				.customize(|bundle| bundle.set_use_isolating(false))
				.build()
				.map_err(|e| Error::string(&e.to_string()))?,
		);
		info!("locales loaded");

		engines::TeraView::build()?.post_process(move |tera| {
			tera.register_function("t", FluentLoader::new(arc.clone()));
			Ok(())
		})?
	} else {
		engines::TeraView::build()?
	};

	Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
}
```
## v0.16.2
- Update auth import in the Authentication document. [https://github.com/loco-rs/loco/pull/1531](https://github.com/loco-rs/loco/pull/1531)
- Adding cache control header to the static asset middleware. [https://github.com/loco-rs/loco/pull/1535](https://github.com/loco-rs/loco/pull/1535)
- Fix borrow checker when sending config to handle_job_command when feature with-db is off. [https://github.com/loco-rs/loco/pull/1536](https://github.com/loco-rs/loco/pull/1536)
- feat: add initializer health checks to doctor command. [https://github.com/loco-rs/loco/pull/1537](https://github.com/loco-rs/loco/pull/1537)
- Update shuttle template to 0.56. [https://github.com/loco-rs/loco/pull/1518](https://github.com/loco-rs/loco/pull/1518)
- Encapsulate post-processing into Tera engine creation. [https://github.com/loco-rs/loco/pull/1540](https://github.com/loco-rs/loco/pull/1540)
- Adding QueryValidateWithMessage. [https://github.com/loco-rs/loco/pull/1521](https://github.com/loco-rs/loco/pull/1521)
- Add S3 driver with credentials and endpoint support. [https://github.com/loco-rs/loco/pull/1539](https://github.com/loco-rs/loco/pull/1539)

## v0.16.1
- fix clippy result_large_err. [https://github.com/loco-rs/loco/pull/1496](https://github.com/loco-rs/loco/pull/1496)
- chore: remove async-std. [https://github.com/loco-rs/loco/pull/1492](https://github.com/loco-rs/loco/pull/1492)
- fix: Bump shuttle version to 0.55.0. [https://github.com/loco-rs/loco/pull/1488](https://github.com/loco-rs/loco/pull/1488)
- Change the Docker building image to 1.87. [https://github.com/loco-rs/loco/pull/1475](https://github.com/loco-rs/loco/pull/1475)
- Fix Clippy warnings for Rust 1.88 stable. [https://github.com/loco-rs/loco/pull/1519](https://github.com/loco-rs/loco/pull/1519) 
- Remove Migrator from boot_test_* doc comments. [https://github.com/loco-rs/loco/pull/1512](https://github.com/loco-rs/loco/pull/1512) 
- fix: use rust-lld linker on Windows. [https://github.com/loco-rs/loco/pull/1508](https://github.com/loco-rs/loco/pull/1508) 
- Fix precompressed in static assets. [https://github.com/loco-rs/loco/pull/1524](https://github.com/loco-rs/loco/pull/1524) 
- Support multiple JWT locations. [https://github.com/loco-rs/loco/pull/1497](https://github.com/loco-rs/loco/pull/1497) 

## v0.16.0

**Note:** For detailed upgrade steps for breaking changes, see the [upgrade guide](https://loco.rs/docs/extras/upgrades/#upgrade-from-0-15-x-to-0-16-x).

- chore: improve readability and performance by using map_err in Model. [https://github.com/loco-rs/loco/pull/1311](https://github.com/loco-rs/loco/pull/1311)
- Allow testing the controller by passing a cookie. [https://github.com/loco-rs/loco/pull/1326](https://github.com/loco-rs/loco/pull/1326)
- Support BigInt in the scaffold Array. [https://github.com/loco-rs/loco/pull/1304](https://github.com/loco-rs/loco/pull/1304)
- Add `escape` Tera function to the scaffold list template. [https://github.com/loco-rs/loco/pull/1337](https://github.com/loco-rs/loco/pull/1337)
- Return a specific error when logging in with a non-existent email. [https://github.com/loco-rs/loco/pull/1336](https://github.com/loco-rs/loco/pull/1336)
- Return a specific error when trying to verify with an invalid token. [https://github.com/loco-rs/loco/pull/1340](https://github.com/loco-rs/loco/pull/1340)
- Clippy 1.86. [https://github.com/loco-rs/loco/pull/1353](https://github.com/loco-rs/loco/pull/1353)
- Fix the DB creation. [https://github.com/loco-rs/loco/pull/1352](https://github.com/loco-rs/loco/pull/1352)
- YAML responses. [https://github.com/loco-rs/loco/pull/1360](https://github.com/loco-rs/loco/pull/1360)
- Swap to the validators' built-in email validation. [https://github.com/loco-rs/loco/pull/1359](https://github.com/loco-rs/loco/pull/1359)
- Cancellation tokens for the Postgres and SQLite background workers. [https://github.com/loco-rs/loco/pull/1365](https://github.com/loco-rs/loco/pull/1365)
- docs: testing auth routes. [https://github.com/loco-rs/loco/pull/1303](https://github.com/loco-rs/loco/pull/1303)
- Add comprehensive tests for the task module. [https://github.com/loco-rs/loco/pull/1386](https://github.com/loco-rs/loco/pull/1386)
- Add comprehensive test coverage for the data module. [https://github.com/loco-rs/loco/pull/1387](https://github.com/loco-rs/loco/pull/1387)
- Add validator extractors test suite. [https://github.com/loco-rs/loco/pull/1388](https://github.com/loco-rs/loco/pull/1388)
- **Breaking changes** Replace sidekiq job management: existing Redis jobs incompatible. [https://github.com/loco-rs/loco/pull/1384](https://github.com/loco-rs/loco/pull/1384)
- **Breaking changes** Add generic type support to the Cache API: cache method calls need type parameters. [https://github.com/loco-rs/loco/pull/1385](https://github.com/loco-rs/loco/pull/1385)
- Adding cache redis driver + configuration instead of enabling from code. [https://github.com/loco-rs/loco/pull/1389](https://github.com/loco-rs/loco/pull/1389)
- Ability to configure pragma for SQLite. [https://github.com/loco-rs/loco/pull/1346](https://github.com/loco-rs/loco/pull/1346)
- **Breaking changes** swap to validators builtin email validation: custom email validator syntax changed. [https://github.com/loco-rs/loco/pull/1359](https://github.com/loco-rs/loco/pull/1359)
- Optimize worker tag filtering string handling. [https://github.com/loco-rs/loco/pull/1396](https://github.com/loco-rs/loco/pull/1396)
- Add test coverage for db.rs. [https://github.com/loco-rs/loco/pull/1400](https://github.com/loco-rs/loco/pull/1400)
- Allow storage of arbitrary custom objects in AppContext. [https://github.com/loco-rs/loco/pull/1404](https://github.com/loco-rs/loco/pull/1404)
- Improve deployment generator CLI. [https://github.com/loco-rs/loco/pull/1413](https://github.com/loco-rs/loco/pull/1413)
- Move auth and validate to the extractor folder. [https://github.com/loco-rs/loco/pull/1414](https://github.com/loco-rs/loco/pull/1414)
- Hot reload on extended Tera templates. [https://github.com/loco-rs/loco/pull/1416](https://github.com/loco-rs/loco/pull/1416)
- **Breaking changes** Update the `init_logger` to use `AppContext` instead of config: function signature changed. [https://github.com/loco-rs/loco/pull/1418](https://github.com/loco-rs/loco/pull/1418)
- Support embedded assets. [https://github.com/loco-rs/loco/pull/1427](https://github.com/loco-rs/loco/pull/1427)
- **Removed dependencies:**
  - [`hyper`](https://github.com/loco-rs/loco/pull/1430)
  - [`thousands`](https://github.com/loco-rs/loco/pull/1431)
  - [`cfg-if`](https://github.com/loco-rs/loco/pull/1432)
  - [`reqwest`](https://github.com/loco-rs/loco/pull/1434)
  - [`serde_variant`](https://github.com/loco-rs/loco/pull/1493)

* **Dependency updates:**
  - Bumped [`tokio`] to `1.45` and [`tokio-util`] to `0.7` ([#1435](https://github.com/loco-rs/loco/pull/1435))
  - Bumped [`colored`] to `3.0` ([#1437](https://github.com/loco-rs/loco/pull/1437))
  - Bumped [`rand`] to `0.9` ([#1439](https://github.com/loco-rs/loco/pull/1439))
  - Bumped [`duct`] to `1.0` ([#1438](https://github.com/loco-rs/loco/pull/1438))
  - Bumped [`redis`] to `0.31`, [`bb8`] to `0.9`, and [`bb8-redis`] to `0.23` ([commit `7e7be`](https://github.com/loco-rs/loco/commit/7e7bebe15f74c377c93d979aab41c52eb871d667))
  - Updated Loco template crates ([#1440](https://github.com/loco-rs/loco/pull/1440))

- Support custom flags from `sea-orm entity`. [https://github.com/loco-rs/loco/pull/1442](https://github.com/loco-rs/loco/pull/1442)
- Better `loco new` cleanup folders. [https://github.com/loco-rs/loco/pull/1429](https://github.com/loco-rs/loco/pull/1429)
- Remove legacy mailer derive macro code. [https://github.com/loco-rs/loco/pull/1472](https://github.com/loco-rs/loco/pull/1472)
- Make extract_token and get_jwt_from_config fn public. [https://github.com/loco-rs/loco/pull/1495](https://github.com/loco-rs/loco/pull/1495)

## v0.15.0

- Added total_items to pagination view & response. [https://github.com/loco-rs/loco/pull/1197](https://github.com/loco-rs/loco/pull/1197)
- Flatten (de)serialization of custom user claims. [https://github.com/loco-rs/loco/pull/1159](https://github.com/loco-rs/loco/pull/1159)
- Updated validator to 0.20. [https://github.com/loco-rs/loco/pull/1199](https://github.com/loco-rs/loco/pull/1199)
- Scaffold v2. [https://github.com/loco-rs/loco/pull/1209](https://github.com/loco-rs/loco/pull/1209)
- Fix generator Docker deployment to support both server-side and client-side rendering. [https://github.com/loco-rs/loco/pull/1227](https://github.com/loco-rs/loco/pull/1227)
- Docs: num_workers worker configuration. [https://github.com/loco-rs/loco/pull/1242](https://github.com/loco-rs/loco/pull/1242)
- Smoother model validations. [https://github.com/loco-rs/loco/pull/1233](https://github.com/loco-rs/loco/pull/1233)
- Docs: num_workers worker configuration. [https://github.com/loco-rs/loco/pull/1242](https://github.com/loco-rs/loco/pull/1242)
- Ignore SQLite WAL and SHM files and update Cargo watch crate docs. [https://github.com/loco-rs/loco/pull/1254](https://github.com/loco-rs/loco/pull/1254)
- Remove fs-err crate. [https://github.com/loco-rs/loco/pull/1253](https://github.com/loco-rs/loco/pull/1253)
- Allows to run scheduler as part of cargo loco start. [https://github.com/loco-rs/loco/pull/1247](https://github.com/loco-rs/loco/pull/1247)
- Added prefix and route nesting to AppRoutes. [https://github.com/loco-rs/loco/pull/1241](https://github.com/loco-rs/loco/pull/1241)
- Replace hyper crate with axum. [https://github.com/loco-rs/loco/pull/1258](https://github.com/loco-rs/loco/pull/1258)
- Remove mime crate. [https://github.com/loco-rs/loco/pull/1256](https://github.com/loco-rs/loco/pull/1256)
- Support async tests. [https://github.com/loco-rs/loco/pull/1237](https://github.com/loco-rs/loco/pull/1237)
- Change job queue status from cli. [https://github.com/loco-rs/loco/pull/1228](https://github.com/loco-rs/loco/pull/1228)
- Handle panics in queue worker. [https://github.com/loco-rs/loco/pull/1274](https://github.com/loco-rs/loco/pull/1274)
- Schema with defaults. [https://github.com/loco-rs/loco/pull/1273](https://github.com/loco-rs/loco/pull/1273)
- Add data subsystem. [https://github.com/loco-rs/loco/pull/1267](https://github.com/loco-rs/loco/pull/1267)
- Add "endpoint" arg to azure storage builder.[https://github.com/loco-rs/loco/pull/1317](https://github.com/loco-rs/loco/pull/1317)
- Improve readability and performance by using map_err in Model. [https://github.com/loco-rs/loco/pull/1311](https://github.com/loco-rs/loco/pull/1311)

### Breaking Changes

In module `loco_rs::auth::jwt` in struct `JWT`, the impl method `generate_token` signature has changed.
Migration:

Before

```rust
jwt.generate_token(&expiration, pid.clone(), None);
```

After

```rust
jwt.generate_token(expiration, pid.clone(), Map::new());
//                 ^ no "&"                 ^ serde_json::map (doesn't allocate in constructor)
```

## v0.14.1

- Fix: bump shuttle to 0.51.0. [https://github.com/loco-rs/loco/pull/1169](https://github.com/loco-rs/loco/pull/1169)
- Return 422 status code for JSON rejection errors. [https://github.com/loco-rs/loco/pull/1173](https://github.com/loco-rs/loco/pull/1173)
- Address clippy warnings for Rust stable 1.84. [https://github.com/loco-rs/loco/pull/1168](https://github.com/loco-rs/loco/pull/1168)
- Bump shuttle to 0.51.0. [https://github.com/loco-rs/loco/pull/1169](https://github.com/loco-rs/loco/pull/1169)
- Return 422 status code for JSON rejection errors. [https://github.com/loco-rs/loco/pull/1173](https://github.com/loco-rs/loco/pull/1173)
- Return json validation details response. [https://github.com/loco-rs/loco/pull/1174](https://github.com/loco-rs/loco/pull/1174)
- Fix example command after generating schedule. [https://github.com/loco-rs/loco/pull/1176](https://github.com/loco-rs/loco/pull/1176)
- Fixed independent features. [https://github.com/loco-rs/loco/pull/1177](https://github.com/loco-rs/loco/pull/1177)
- Custom response header for redirect. [https://github.com/loco-rs/loco/pull/1186](https://github.com/loco-rs/loco/pull/1186)
- Added run_on_start feature to scheduler. [https://github.com/loco-rs/loco/pull/1184](https://github.com/loco-rs/loco/pull/1184)
- feat: public jwt extractor from non-mutable reference to parts. [https://github.com/loco-rs/loco/pull/1190](https://github.com/loco-rs/loco/pull/1190)

## v0.14

- feat: smart migration generator. you can now generate migration based on naming them for creating a table, adding columns, references, join tables and more. [https://github.com/loco-rs/loco/pull/1086](https://github.com/loco-rs/loco/pull/1086)
- feat: `cargo loco routes` will now pretty-print routes
- fix: guard jwt error behind feature flag. [https://github.com/loco-rs/loco/pull/1032](https://github.com/loco-rs/loco/pull/1032)
- fix: logger file_appender not using the seperated format setting. [https://github.com/loco-rs/loco/pull/1036](https://github.com/loco-rs/loco/pull/1036)
- seed cli command. [https://github.com/loco-rs/loco/pull/1046](https://github.com/loco-rs/loco/pull/1046)
- Updated validator to 0.19. [https://github.com/loco-rs/loco/pull/993](https://github.com/loco-rs/loco/pull/993)
  ### Breaking Changes
  Bump validator to 0.19 in your local `Cargo.toml`
- Testing helpers: simplified function calls + adding html selector. [https://github.com/loco-rs/loco/pull/1047](https://github.com/loco-rs/loco/pull/1047)

  ### Breaking Changes

  #### Updated Import Paths

  The testing module import path has been updated. To adapt your code, update imports from:

  ```rust
  use loco_rs::testing;
  ```

  to:

  ```rust
  use testing::prelude::*;
  ```

  #### Simplified Function Calls

  Function calls within the testing module no longer require the testing:: prefix. Update your code accordingly. For example:

  Before:

  ```rust
  let boot = testing::boot_test::<App>().await.unwrap();
  ```

  After:

  ```rust
  let boot = boot_test::<App>().await.unwrap();
  ```

- implement commands to manage background jobs. [https://github.com/loco-rs/loco/pull/1071](https://github.com/loco-rs/loco/pull/1071)
- magic link. [https://github.com/loco-rs/loco/pull/1085](https://github.com/loco-rs/loco/pull/1085)
- infer migration. [https://github.com/loco-rs/loco/pull/1086](https://github.com/loco-rs/loco/pull/1086)
- Remove unnecessary calls to 'register_tasks' functions in scheduler. [https://github.com/loco-rs/loco/pull/1100](https://github.com/loco-rs/loco/pull/1100)
- implement commands to manage background jobs. [https://github.com/loco-rs/loco/pull/1071](https://github.com/loco-rs/loco/pull/1071)
- expose hello_name for SMTP client config. [https://github.com/loco-rs/loco/pull/1057](https://github.com/loco-rs/loco/pull/1057)
- use reqwest with rustls rather than openssl. [https://github.com/loco-rs/loco/pull/1058](https://github.com/loco-rs/loco/pull/1058)
- more flexible config, take more values from ENV. [https://github.com/loco-rs/loco/pull/1058](https://github.com/loco-rs/loco/pull/1058)
- refactor: Use opendal to replace object_store. [https://github.com/loco-rs/loco/pull/897](https://github.com/loco-rs/loco/pull/897)
- allow override loco template. [https://github.com/loco-rs/loco/pull/1102](https://github.com/loco-rs/loco/pull/1102)
- support custom config folder. [https://github.com/loco-rs/loco/pull/1081](https://github.com/loco-rs/loco/pull/1081)
- feat: upgrade to Axum 8. [https://github.com/loco-rs/loco/pull/1130](https://github.com/loco-rs/loco/pull/1130)
- create load config hook. [https://github.com/loco-rs/loco/pull/1143](https://github.com/loco-rs/loco/pull/1143)
- initial impl new migration dsl. [https://github.com/loco-rs/loco/pull/1125](https://github.com/loco-rs/loco/pull/1125)
- allow disable limit_payload middleware. [https://github.com/loco-rs/loco/pull/1113](https://github.com/loco-rs/loco/pull/1113)

## v0.13.2

- static fallback now returns 200 and not 404 [https://github.com/loco-rs/loco/pull/991](https://github.com/loco-rs/loco/pull/991)
- cache system now has expiry [https://github.com/loco-rs/loco/pull/1006](https://github.com/loco-rs/loco/pull/1006)
- fixed: http interface binding [https://github.com/loco-rs/loco/pull/1007](https://github.com/loco-rs/loco/pull/1007)
- JWT claims now editable and public [https://github.com/loco-rs/loco/issues/988](https://github.com/loco-rs/loco/issues/988)
- CORS now not enabled in dev mode to avoid friction [https://github.com/loco-rs/loco/pull/1009](https://github.com/loco-rs/loco/pull/1009)
- fixed: task code generation now injects in all cases [https://github.com/loco-rs/loco/pull/1012](https://github.com/loco-rs/loco/pull/1012)

**BREAKING**
In your `app.rs` add the following injection comment at the bottom:

```rust
fn register_tasks(tasks: &mut Tasks) {
    tasks.register(tasks::user_report::UserReport);
    tasks.register(tasks::seed::SeedData);
    tasks.register(tasks::foo::Foo);
    // tasks-inject (do not remove)
}
```

- fix: seeding now sets autoincrement fields in the relevant DBs [https://github.com/loco-rs/loco/pull/1014](https://github.com/loco-rs/loco/pull/1014)
- fix: avoid generating entities from queue tables when the queue backend is database based [https://github.com/loco-rs/loco/issues/1013](https://github.com/loco-rs/loco/issues/1013)
- removed: channels moved to an initializer [https://github.com/loco-rs/loco/issues/892](https://github.com/loco-rs/loco/issues/892)
  **BREAKING**
  See how this looks like in [https://github.com/loco-rs/chat-rooms](https://github.com/loco-rs/chat-rooms)

## v0.13.0

- Added SQLite background job support [https://github.com/loco-rs/loco/pull/969](https://github.com/loco-rs/loco/pull/969)
- Added automatic updating of `updated_at` on change [https://github.com/loco-rs/loco/pull/962](https://github.com/loco-rs/loco/pull/962)
- fixed codegen injection point in migrations [https://github.com/loco-rs/loco/pull/952](https://github.com/loco-rs/loco/pull/952)

**NOTE: update your migration listing module like so:**

```rust
// migrations/src/lib.rs
  vec![
      Box::new(m20220101_000001_users::Migration),
      Box::new(m20231103_114510_notes::Migration),
      Box::new(m20240416_071825_roles::Migration),
      Box::new(m20240416_082115_users_roles::Migration),
      // inject-above (do not remove this comment)
  ]
```

Add the comment just before the closing array (`inject-above`)

- Added ability to name references in [https://github.com/loco-rs/loco/pull/955](https://github.com/loco-rs/loco/pull/955):

```sh
$ generate scaffold posts title:string! content:string! written_by:references:users approved_by:references:users
```

- Added hot-reload like experience to Tera templates [https://github.com/loco-rs/loco/issues/977](https://github.com/loco-rs/loco/issues/977), in debug builds only.

**NOTE: update your initializers `after_routes` like so:**

```rust
// src/initializers/view_engine.rs
async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
    #[allow(unused_mut)]
    let mut tera_engine = engines::TeraView::build()?;
    if std::path::Path::new(I18N_DIR).exists() {
        let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
            .shared_resources(Some(&[I18N_SHARED.into()]))
            .customize(|bundle| bundle.set_use_isolating(false))
            .build()
            .map_err(|e| Error::string(&e.to_string()))?;
        #[cfg(debug_assertions)]
        tera_engine
            .tera
            .lock()
            .expect("lock")
            .register_function("t", FluentLoader::new(arc));

        #[cfg(not(debug_assertions))]
        tera_engine
            .tera
            .register_function("t", FluentLoader::new(arc));
        info!("locales loaded");
    }

    Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
}
```

- `loco doctor` now checks for app-specific minimum dependency versions. This should help in upgrades. `doctor` also supports "production only" checks which you can run in production with `loco doctor --production`. This, for example, will check your connections but will not check dependencies. [https://github.com/loco-rs/loco/pull/931](https://github.com/loco-rs/loco/pull/931)
- Use a single loco-rs dep for a whole project. [https://github.com/loco-rs/loco/pull/927](https://github.com/loco-rs/loco/pull/927)
- chore: fix generated testcase. [https://github.com/loco-rs/loco/pull/939](https://github.com/loco-rs/loco/pull/939)
- chore: Correct cargo test message. [https://github.com/loco-rs/loco/pull/938](https://github.com/loco-rs/loco/pull/938)
- Add relevant meta tags for better defaults. [https://github.com/loco-rs/loco/pull/943](https://github.com/loco-rs/loco/pull/943)
- Update cli message with correct command. [https://github.com/loco-rs/loco/pull/942](https://github.com/loco-rs/loco/pull/942)
- remove lazy_static. [https://github.com/loco-rs/loco/pull/941](https://github.com/loco-rs/loco/pull/941)
- change update HTTP verb semantics to put+patch. [https://github.com/loco-rs/loco/pull/919](https://github.com/loco-rs/loco/pull/919)
- Fixed HTML scaffold error. [https://github.com/loco-rs/loco/pull/960](https://github.com/loco-rs/loco/pull/960)
- Scaffolded HTML update method should be POST. [https://github.com/loco-rs/loco/pull/963](https://github.com/loco-rs/loco/pull/963)

## v0.12.0

This release have been primarily about cleanups and simplification.

Please update:

- `loco-rs`
- `loco-cli`

Changes:

- **generators (BREAKING)**: all prefixes in starters (e.g. `/api`) are now _local to each controller_, and generators will be prefix-aware (`--api` generator will add an `/api` prefix to controllers) [https://github.com/loco-rs/loco/pull/818](https://github.com/loco-rs/loco/pull/818)

To migrate, please move prefixes from `app.rs` to each controller you use in `controllers/`, for example in `notes` controller:

```rust
Routes::new()
    .prefix("api/notes")
    .add("/", get(list))
```

- **starters**: removed `.devcontainer` which can now be found in [loco-devcontainer](https://github.com/loco-rs/loco-devcontainer)
- **starters**: removed example `notes` scaffold (model, controllers, etc), and unified `user` and `auth` into a single file: `auth.rs`
- **generators**: `scaffold` generator will now generate a CRUD with `PUT` and `PATCH` semantics for updating an entity [https://github.com/loco-rs/loco/issues/896](https://github.com/loco-rs/loco/issues/896)
- **cleanup**: `loco-extras` was moved out of the repo, but we've incorporated `MultiDB` and `ExtraDB` from `extras` into `loco-rs` [https://github.com/loco-rs/loco/pull/917](https://github.com/loco-rs/loco/pull/917)

- `cargo loco doctor` now checks for minimal required SeaORM CLI version
- **BREAKING** Improved migration generator. If you have an existing migration project, add the following comment indicator to the top of the `vec` statement and right below the opening bracked like so in `migration/src/lib.rs`:

```rust
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // inject-below (do not remove this comment)
```

## v0.11.0

- Upgrade **SeaORM to v1.1.0**
- Added OpenAPI example
- Improve health route [https://github.com/loco-rs/loco/pull/851](https://github.com/loco-rs/loco/pull/851)
- Add good pragmas to Sqlite [https://github.com/loco-rs/loco/pull/848](https://github.com/loco-rs/loco/pull/848)
- Upgrade to rsbuild 1.0. [https://github.com/loco-rs/loco/pull/792](https://github.com/loco-rs/loco/pull/792)
- Implements fmt::Debug to pub structs. [https://github.com/loco-rs/loco/pull/812](https://github.com/loco-rs/loco/pull/812)
- Add num_workers config for sidekiq queue. [https://github.com/loco-rs/loco/pull/823](https://github.com/loco-rs/loco/pull/823)
- Fix some comments in the starters and example code. [https://github.com/loco-rs/loco/pull/824](https://github.com/loco-rs/loco/pull/824)
- Fix Y2038 bug for JWT on 32 bit platforms. [https://github.com/loco-rs/loco/pull/825](https://github.com/loco-rs/loco/pull/825)
- Make App URL in Boot Banner Clickable. [https://github.com/loco-rs/loco/pull/826](https://github.com/loco-rs/loco/pull/826)
- Add `--no-banner` flag to allow disabling the banner display. [https://github.com/loco-rs/loco/pull/839](https://github.com/loco-rs/loco/pull/839)
- add on_shutdown hook. [https://github.com/loco-rs/loco/pull/842](https://github.com/loco-rs/loco/pull/842)

## v0.10.1

- `Format(respond_to): Format` extractor in controller can now be replaced with `respond_to: RespondTo` extractor for less typing.
- When supplying data to views, you can now use `data!` instead of `serde_json::json!` for shorthand.
- Refactor middlewares. [https://github.com/loco-rs/loco/pull/785](https://github.com/loco-rs/loco/pull/785). Middleware selection, configuration, and tweaking is MUCH more powerful and convenient now. You can keep the `middleware:` section empty or remove it now, see more in [the middleware docs](https://loco.rs/docs/the-app/controller/#middleware)
- **NEW (BREAKING)** background worker subsystem is now queue agnostic. Providing for both Redis and Postgres with a change of configuration. This means you can now use a full-Postgres stack to remove Redis as a dependency if you wish. Here are steps to migrate your codebase:

```rust
// in your app.rs, change the worker registration code:

// BEFORE
fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
    p.register(DownloadWorker::build(ctx));
}

// AFTER
async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()>{
    queue.register(DownloadWorker::build(ctx)).await?;
    Ok(())
}

// in your app.rs, replace the `worker` module references.
// REMOVE
worker::{AppWorker, Processor},
// REPLACE WITH
bgworker::{BackgroundWorker, Queue},

// in your workers change the signature, and add the `build` function

// BEFORE
impl worker::Worker<DownloadWorkerArgs> for DownloadWorker {
    async fn perform(&self, args: DownloadWorkerArgs) -> worker::Result<()> {

// AFTER
#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {

// Finally, remove the `AppWorker` trait implementation completely.

// REMOVE
impl worker::AppWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}
```

Finally, update your `development.yaml` and `test.yaml` with a `kind`:

```yaml
queue:
  kind: Redis # add this to the existing `queue` section
```

- **UPGRADED (BREAKING)**: `validator` crate was upgraded which require some small tweaks to work with the new API:

```rust
// BEFORE:
#[validate(custom = "validation::is_valid_email")]
pub email: String,

// AFTER:
#[validate(custom (function = "validation::is_valid_email"))]
pub email: String,
```

Then update your `Cargo.toml` to take version `0.18`:

```toml
# update
validator = { version = "0.18" }
```

- **UPGRADED (BREAKING)**: `axum-test` crate was upgraded
  Update your `Cargo.toml` to version `16`:

```toml
# update
axum-test = { version = "16" }
```

## v0.9.0

- Add fallback behavior. [https://github.com/loco-rs/loco/pull/732](https://github.com/loco-rs/loco/pull/732)
- Add Scheduler Feature for Running Cron Jobs. [https://github.com/loco-rs/loco/pull/735](https://github.com/loco-rs/loco/pull/735)
- Add `--html`, `--htmx` and `--api` flags to scaffold CLI command. [https://github.com/loco-rs/loco/pull/749](https://github.com/loco-rs/loco/pull/749)
- Add base template for scaffold generation. [https://github.com/loco-rs/loco/pull/752](https://github.com/loco-rs/loco/pull/752)
- Connect Redis only when the worker is BackgroundQueue. [https://github.com/loco-rs/loco/pull/755](https://github.com/loco-rs/loco/pull/755)
- Add loco doctor --config. [https://github.com/loco-rs/loco/pull/736](https://github.com/loco-rs/loco/pull/736)
- Rename demo: blo -> demo_app. [https://github.com/loco-rs/loco/pull/741](https://github.com/loco-rs/loco/pull/741)

## v0.8.1

- fix: introduce secondary binary for compile-and-run on Windows. [https://github.com/loco-rs/loco/pull/727](https://github.com/loco-rs/loco/pull/727)

## v0.8.0

- Added: loco-cli (`loco new`) now receives options from CLI and/or interactively asks for configuration options such as which asset pipeline, background worker type, or database provider to use.
- Fix: custom queue names now merge with default queues.
- Added `remote_ip` middleware for resolving client remote IP when under a proxy or loadbalancer, similar to the Rails `remote_ip` middleware.
- Added `secure_headers` middleware for setting secure headers by default, similar to how [https://github.com/github/secure_headers](https://github.com/github/secure_headers) works. This is now ON by default to promote security-by-default.
- Added: `money`, `blob` types to entitie generator.

## 0.7.0

- Moving to _timezone aware timestamps_. From now on migrations will generate **timestamps with time zone** by default. Moving to TZ aware timestamps in combination with newly revamped timestamp code generation in SeaORM v1.0.0 finally allows for _seamlessly_ moving between using `sqlite` and `postgres` with minimal or no entities code changes (resolved [this long standing issue](https://github.com/loco-rs/loco/issues/518#issuecomment-2051708319)). TZ aware timestamps also aligns us with how Rails works today (initially Rails had a no-tz timestamps, and today the default is to use timestamps). If not specified the TZ is the server TZ, which is usually UTC, therefore semantically this is almost like a no-tz timestamp.

**A few highlights:**

Generated entities will now always use `DateTimeWithTimeZone` for the default timestamp fields:

```
...
Generating users.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
...
```

For better cross database provider compatibility, from now on prefer the `tstz` type instead of just `ts` when using generators (i.e. `cargo loco generate model movie released:tstz`)

- remove eyer lib. [https://github.com/loco-rs/loco/pull/650](https://github.com/loco-rs/loco/pull/650)

  ### Breaking Changes:

  1.  Update the Main Function in src/bin/main

      Replace the return type of the main function:

      **Before:**

      ```rust
      async fn main() -> eyre::Result<()>
      ```

      **After:**

      ```rust
      async fn main() -> loco_rs::Result<()>
      ```

  2.  Modify examples/playground.rs
      You need to apply two changes here:

          a. Update the Function Signature
          **Before:**

          ```rust
          async fn main() -> eyre::Result<()>
          ```

          **After:**

          ```rust
          async fn main() -> loco_rs::Result<()>
          ```

          b. Adjust the Context Handling
          **Before:**

          ```rust
          let _ctx = playground::<App>().await.context("playground")?;
          ```

          **After:**

          ```rust
          let _ctx = playground::<App>().await?;
          ```

      Note,
      If you are using eyre in your project, you can continue to do so. We have only removed this crate from our base code dependencies.

- Bump rstest crate to 0.21.0. [https://github.com/loco-rs/loco/pull/650](https://github.com/loco-rs/loco/pull/650)
- Bump serial_test crate to 3.1.1. [https://github.com/loco-rs/loco/pull/651](https://github.com/loco-rs/loco/pull/651)
- Bumo object store to create to 0.10.2. [https://github.com/loco-rs/loco/pull/654](https://github.com/loco-rs/loco/pull/654)
- Bump axum crate to 0.7.5. [https://github.com/loco-rs/loco/pull/652](https://github.com/loco-rs/loco/pull/652)
- Add Hooks::before_routes to give user control over initial axum::Router construction. [https://github.com/loco-rs/loco/pull/646](https://github.com/loco-rs/loco/pull/646)
- Support logger file appender. [https://github.com/loco-rs/loco/pull/636](https://github.com/loco-rs/loco/pull/636)
- Response from the template. [https://github.com/loco-rs/loco/pull/682](https://github.com/loco-rs/loco/pull/682)
- Add get_or_insert function to cache layer. [https://github.com/loco-rs/loco/pull/637](https://github.com/loco-rs/loco/pull/637)
- Bump ORM create to 1.0.0. [https://github.com/loco-rs/loco/pull/684](https://github.com/loco-rs/loco/pull/684)

## 0.6.2

- Use Rust-based tooling for SaaS starter frontend. [https://github.com/loco-rs/loco/pull/625](https://github.com/loco-rs/loco/pull/625)
- Default binding to localhost to avoid firewall dialogues during development on macOS. [https://github.com/loco-rs/loco/pull/627](https://github.com/loco-rs/loco/pull/627)
- upgrade sea-orm to 1.0.0 RC 7. [https://github.com/loco-rs/loco/pull/627](https://github.com/loco-rs/loco/pull/639)
- Add a down migration command. [https://github.com/loco-rs/loco/pull/414](https://github.com/loco-rs/loco/pull/414)
- replace create_postgres_database function table_name to db_name. [https://github.com/loco-rs/loco/pull/647](https://github.com/loco-rs/loco/pull/647)

## 0.6.1

- Upgrade htmx generator to htmx2. [https://github.com/loco-rs/loco/pull/629](https://github.com/loco-rs/loco/pull/629)

## 0.6.0 https://github.com/loco-rs/loco/pull/610

- Bump socketioxide to v0.13.1. [https://github.com/loco-rs/loco/pull/594](https://github.com/loco-rs/loco/pull/594)
- Add CC and BCC fields to the mailers. [https://github.com/loco-rs/loco/pull/599](https://github.com/loco-rs/loco/pull/599)
- Delete reset tokens after use. [https://github.com/loco-rs/loco/pull/602](https://github.com/loco-rs/loco/pull/602)
- Generator html support delete entity. [https://github.com/loco-rs/loco/pull/604](https://github.com/loco-rs/loco/pull/604)
- **Breaking changes** move task args from BTreeMap to struct. [https://github.com/loco-rs/loco/pull/609](https://github.com/loco-rs/loco/pull/609)
  - Change task signature from `async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>)` to `async fn run(&self, _app_context: &AppContext, _vars: &task::Vars) -> Result<()>`
  - **Breaking changes** change default port to 5150. [https://github.com/loco-rs/loco/pull/611](https://github.com/loco-rs/loco/pull/611)
- Update shuttle version in deployment generation. [https://github.com/loco-rs/loco/pull/616](https://github.com/loco-rs/loco/pull/616)

## v0.5.0 https://github.com/loco-rs/loco/pull/593

- refactor auth middleware for supporting bearer, cookie and query. [https://github.com/loco-rs/loco/pull/560](https://github.com/loco-rs/loco/pull/560)
- SeaORM upgraded: `rc1` -> `rc4`. [https://github.com/loco-rs/loco/pull/585](https://github.com/loco-rs/loco/pull/585)
- Adding Cache to app content. [https://github.com/loco-rs/loco/pull/570](https://github.com/loco-rs/loco/pull/570)
- Apply a layer to a specific handler using `layer` method. [https://github.com/loco-rs/loco/pull/554](https://github.com/loco-rs/loco/pull/554)
- Add the debug macro to the templates to improve the errors. [https://github.com/loco-rs/loco/pull/547](https://github.com/loco-rs/loco/pull/547)
- Opentelemetry initializer. [https://github.com/loco-rs/loco/pull/531](https://github.com/loco-rs/loco/pull/531)
- Refactor auth middleware for supporting bearer, cookie and query [https://github.com/loco-rs/loco/pull/560](https://github.com/loco-rs/loco/pull/560)
- Add redirect response [https://github.com/loco-rs/loco/pull/563](https://github.com/loco-rs/loco/pull/563)
- **Breaking changes** Adding a custom claims `Option<serde_json::Value>` to the `UserClaims` struct (type changed). [https://github.com/loco-rs/loco/pull/578](https://github.com/loco-rs/loco/pull/578)
- **Breaking changes** Refactored DSL and Pagination: namespace changes. [https://github.com/loco-rs/loco/pull/566](https://github.com/loco-rs/loco/pull/566)
  - Replaced `model::query::dsl::` with `model::query`.
  - Replaced `model::query::exec::paginate` with `model::query::paginate`.
  - Updated the `PaginatedResponse` struct. Refer to its usage example [here](https://github.com/loco-rs/loco/blob/master/examples/demo/src/views/notes.rs#L29).
- **Breaking changes** When introducing the Cache system which is much more flexible than having just Redis, we now call the 'redis' member simply a 'queue' which indicates it should be used only for the internal queue and not as a general purpose cache. In the application configuration setting `redis`, change to `queue`. [https://github.com/loco-rs/loco/pull/590](https://github.com/loco-rs/loco/pull/590)

```yaml
# before:
redis:
# after:
queue:
```

- **Breaking changes** We have made a few parts of the context pluggable, such as the `storage` and new `cache` subsystems, this is why we decided to let you configure the context entirely before starting up your app. As a result, if you have a storage building hook code it should move to `after_context`, see example [here](https://github.com/loco-rs/loco/pull/570/files#diff-5534e8826fb82e5c7f2587d270a51b48009341e79889d1504e6b63b2f0b652bdR83). [https://github.com/loco-rs/loco/pull/570](https://github.com/loco-rs/loco/pull/570)

## v0.4.0

- Refactored model validation for better developer experience. Added a few traits and structs to `loco::prelude` for a smoother import story. Introducing `Validatable`:

```rust
impl Validatable for super::_entities::users::ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

// now you can call `user.validate()` freely
```

- Refactored type field mapping to be centralized. Now model, scaffold share the same field mapping, so no more gaps like [https://github.com/loco-rs/loco/issues/513](https://github.com/loco-rs/loco/issues/513) (e.g. when calling `loco generate model title:string` the ability to map `string` into something useful in the code generation side)
  **NOTE** the `_integer` class of types are now just `_int`, e.g. `big_int`, so that it correlate with the `int` field name in a better way

- Adding to to quiery dsl `is_in` and `is_not_in`. [https://github.com/loco-rs/loco/pull/507](https://github.com/loco-rs/loco/pull/507)
- Added: in your configuration you can now use an `initializers:` section for initializer specific settings

  ```yaml
  # Initializers Configuration
  initializers:
  # oauth2:
  #   authorization_code: # Authorization code grant type
  #     - client_identifier: google # Identifier for the OAuth2 provider. Replace 'google' with your provider's name if different, must be unique within the oauth2 config.
  #       ... other fields
  ```

- Docs: fix schema data types mapping. [https://github.com/loco-rs/loco/pull/506](https://github.com/loco-rs/loco/pull/506)
- Let Result accept other errors. [https://github.com/loco-rs/loco/pull/505](https://github.com/loco-rs/loco/pull/505)
- Allow trailing slashes in URIs by adding the NormalizePathLayer. [https://github.com/loco-rs/loco/pull/481](https://github.com/loco-rs/loco/pull/481)
- **BREAKING** Move from `Result<impl IntoResponse>` to `Result<Response>`. This enables much greater flexibility building APIs, where with `Result<Response>` you mix and match response types based on custom logic (returning JSON and HTML/String in the same route).
- **Added**: mime responders similar to `respond_to` in Rails:

1. Use the `Format` extractor
2. Match on `respond_to`
3. Create different content for different response formats

The following route will always return JSON, unless explicitly asked for HTML with a
`Content-Type: text/html` (or `Accept: `) header:

```rust
pub async fn get_one(
    Format(respond_to): Format,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    match respond_to {
        RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
        _ => format::json(item),
    }
}
```

## 0.3.2

- Redisgin pagination. [https://github.com/loco-rs/loco/pull/463](https://github.com/loco-rs/loco/pull/463)
- Wrap seaorm query and condition for common use cases. [https://github.com/loco-rs/loco/pull/463](https://github.com/loco-rs/loco/pull/463)
- Adding to loco-extras initializer for extra or multiple db. [https://github.com/loco-rs/loco/pull/471](https://github.com/loco-rs/loco/pull/471)
- Scaffold now supporting different templates such as API,HTML or htmx, this future is in beta.[https://github.com/loco-rs/loco/pull/474](https://github.com/loco-rs/loco/pull/474)
- Fix generatore fields types + adding tests. [https://github.com/loco-rs/loco/pull/459](https://github.com/loco-rs/loco/pull/459)
- Fix channel cors. [https://github.com/loco-rs/loco/pull/430](https://github.com/loco-rs/loco/pull/430)
- Improve auth controller compatibility with frontend [https://github.com/loco-rs/loco/pull/472](https://github.com/loco-rs/loco/pull/472)

## 0.3.1

- **Breaking changes** Upgrade sea-orm to v1.0.0-rc.1. [https://github.com/loco-rs/loco/pull/420](https://github.com/loco-rs/loco/pull/420)
  Needs to update `sea-orm` crate to use `v1.0.0-rc.1` version.
- Implemented file upload support with versatile strategies. [https://github.com/loco-rs/loco/pull/423](https://github.com/loco-rs/loco/pull/423)
- Create a `loco_extra` crate to share common basic implementations. [https://github.com/loco-rs/loco/pull/425](https://github.com/loco-rs/loco/pull/425)
- Update shuttle deployment template to 0.38. [https://github.com/loco-rs/loco/pull/422](https://github.com/loco-rs/loco/pull/422)
- Enhancement: Move the Serve to Hook flow with the ability to override default serve settings. [https://github.com/loco-rs/loco/pull/418](https://github.com/loco-rs/loco/pull/418)
- Avoid cloning sea_query::ColumnDef. [https://github.com/loco-rs/loco/pull/415](https://github.com/loco-rs/loco/pull/415)
- Allow required UUID type in a scaffold. [https://github.com/loco-rs/loco/pull/408](https://github.com/loco-rs/loco/pull/408)
- Cover `SqlxMySqlPoolConnection` in db.rs. [https://github.com/loco-rs/loco/pull/411](https://github.com/loco-rs/loco/pull/411)
- Update worker docs and change default worker mode. [https://github.com/loco-rs/loco/pull/412](https://github.com/loco-rs/loco/pull/412)
- Added server-side view generation through a new `ViewEngine` infrastructure and `Tera` server-side templates: [https://github.com/loco-rs/loco/pull/389](https://github.com/loco-rs/loco/pull/389)
- Added `generate model --migration-only` [https://github.com/loco-rs/loco/issues/400](https://github.com/loco-rs/loco/issues/400)
- Add JSON to scaffold gen. [https://github.com/loco-rs/loco/pull/396](https://github.com/loco-rs/loco/pull/396)
- Add --binding(-b) and --port(-b) to `cargo loco start`.[https://github.com/loco-rs/loco/pull/402](https://github.com/loco-rs/loco/pull/402)

## 0.2.3

- Add: support for [pre-compressed assets](https://github.com/loco-rs/loco/pull/370/files).
- Added: Support socket channels, see working example [here](https://github.com/loco-rs/chat-rooms). [https://github.com/loco-rs/loco/pull/380](https://github.com/loco-rs/loco/pull/380)
- refactor: optimize checking permissions on Postgres. [9416c](https://github.com/loco-rs/loco/commit/9416c5db85a27e3d30471374effec3fe88bf80a2)
- Added: E2E db. [https://github.com/loco-rs/loco/pull/371](https://github.com/loco-rs/loco/pull/371)

## v0.2.2

- fix: public fields in mailer-op. [e51b7e](https://github.com/loco-rs/loco/commit/e51b7e64e7667c519451ac8a8bea574b2c5d4403)
- fix: handle missing db permissions. [e51b7e](https://github.com/loco-rs/loco/commit/e51b7e64e7667c519451ac8a8bea574b2c5d4403)

## v0.2.1

- enable compression for CompressionLayer, not etag. [https://github.com/loco-rs/loco/pull/356](https://github.com/loco-rs/loco/pull/356)
- Fix nullable JSONB column schema definition. [https://github.com/loco-rs/loco/pull/357](https://github.com/loco-rs/loco/pull/357)

## v0.2.0

- Add: Loco now has Initializers ([see the docs](https://loco.rs/docs/the-app/initializers/)). Initializers help you integrate infra into your app in a seamless way, as well as share pieces of setup code between your projects
- Add: an `init_logger` hook in `src/app.rs` for those who want to take ownership of their logging and tracing stack.
- Add: Return a JSON schema when payload json could not serialize to a struct. [https://github.com/loco-rs/loco/pull/343](https://github.com/loco-rs/loco/pull/343)
- Init logger in cli.rs. [https://github.com/loco-rs/loco/pull/338](https://github.com/loco-rs/loco/pull/338)
- Add: return JSON schema in panic HTTP layer. [https://github.com/loco-rs/loco/pull/336](https://github.com/loco-rs/loco/pull/336)
- Add: JSON field support in model generation. [https://github.com/loco-rs/loco/pull/327](https://github.com/loco-rs/loco/pull/327) [https://github.com/loco-rs/loco/pull/332](https://github.com/loco-rs/loco/pull/332)
- Add: float support in model generation. [https://github.com/loco-rs/loco/pull/317](https://github.com/loco-rs/loco/pull/317)
- Fix: conflicting idx definition on M:M migration. [https://github.com/loco-rs/loco/issues/311](https://github.com/loco-rs/loco/issues/311)
- Add: **Breaking changes** Supply `AppContext` to `routes` Hook. Migration steps in `src/app.rs`:

```rust
// src/app.rs: add app context to routes function
impl Hooks for App {
  ...
  fn routes(_ctx: &AppContext) -> AppRoutes;
  ...
}
```

- Add: **Breaking changes** change parameter type from `&str` to `&Environment` in `src/app.rs`

```rust
// src/app.rs: change parameter type for `environment` from `&str` to `&Environment`
impl Hooks for App {
    ...
    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        create_app::<Self>(mode, environment).await
    }
    ...
```

- Added: setting cookies:

```rust
format::render()
    .cookies(&[
        cookie::Cookie::new("foo", "bar"),
        cookie::Cookie::new("baz", "qux"),
    ])?
    .etag("foobar")?
    .json(notes)
```

## v0.1.9

- Adding [pagination](https://loco.rs/docs/the-app/pagination/) on Models. [https://github.com/loco-rs/loco/pull/238](https://github.com/loco-rs/loco/pull/238)
- Adding compression middleware. [https://github.com/loco-rs/loco/pull/205](https://github.com/loco-rs/loco/pull/205)
  Added support for [compression middleware](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html).
  usage:

```yaml
middlewares:
  compression:
    enable: true
```

- Create a new Database from the CLI. [https://github.com/loco-rs/loco/pull/223](https://github.com/loco-rs/loco/pull/223)
- Validate if seaorm CLI is installed before running `cargo loco db entities` and show a better error to the user. [https://github.com/loco-rs/loco/pull/212](https://github.com/loco-rs/loco/pull/212)
- Adding to `saas and `rest-api` starters a redis and DB in GitHub action workflow to allow users work with github action out of the box. [https://github.com/loco-rs/loco/pull/215](https://github.com/loco-rs/loco/pull/215)
- Adding the app name and the environment to the DB name when creating a new starter. [https://github.com/loco-rs/loco/pull/216](https://github.com/loco-rs/loco/pull/216)
- Fix generator when users adding a `created_at` or `update_at` fields. [https://github.com/loco-rs/loco/pull/214](https://github.com/loco-rs/loco/pull/214)
- Add: `format::render` which allows a builder-like formatting, including setting etag and ad-hoc headers
- Add: Etag middleware, enabled by default in starter projects. Once you set an Etag it will check for cache headers and return `304` if needed. To enable etag in your existing project:

```yaml
#...
middlewares:
  etag:
    enable: true
```

usage:

```rust
  format::render()
      .etag("foobar")?
      .json(Entity::find().all(&ctx.db).await?)
```

#### Authentication: Added API Token Authentication!

- See [https://github.com/loco-rs/loco/pull/217](https://github.com/loco-rs/loco/pull/217)
  Now when you generate a `saas starter` or `rest api` starter you will get additional authentication methods for free:

- Added: authentication added -- **api authentication** where each user has an API token in the schema, and you can authenticate with `Bearer` against that user.
- Added: authentication added -- `JWTWithUser` extractor, which is a convenience for resolving the authenticated JWT claims into a current user from database

**migrating an existing codebase**

Add the following to your generated `src/models/user.rs`:

```rust
#[async_trait]
impl Authenticable for super::_entities::users::Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        super::_entities::users::Model::find_by_pid(db, claims_key).await
    }
}
```

Update imports in this file to include `model::Authenticable`:

```rust
use loco_rs::{
    auth, hash,
    model::{Authenticable, ModelError, ModelResult},
    validation,
    validator::Validate,
};
```

## v0.1.8

- Added: `loco version` for getting an operable version string containing logical crate version and git SHA if available: `0.3.0 (<git sha>)`

To migrate to this behavior from earlier versions, it requires adding the following to your `app.rs` app hooks:

```rust
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }
```

Reminder: `loco --version` will give you the current Loco framework which your app was built against and `loco version` gives you your app version.

- Added: `loco generate migration` for adding ad-hoc migrations
- Added: added support in model generator for many-to-many link table generation via `loco generate model --link`
- Docs: added Migration section, added relations documentation 1:M, M:M
- Adding .devcontainer to starter projects [https://github.com/loco-rs/loco/issues/170](https://github.com/loco-rs/loco/issues/170)
- **Braking changes**: Adding `Hooks::boot` application. Migration steps:
  ```rust
  // Load boot::{create_app, BootResult, StartMode} from loco_rs lib
  // Load migration: use migration::Migrator; Only when using DB
  // Adding boot hook with the following code
  impl Hooks for App {
    ...
    async fn boot(mode: StartMode, environment: &str) -> Result<BootResult> {
      // With DB:
      create_app::<Self, Migrator>(mode, environment).await
      // Without DB:
      create_app::<Self>(mode, environment).await
    }
    ...
  }
  ```

## v0.1.7

- Added pretty backtraces [https://github.com/loco-rs/loco/issues/41](https://github.com/loco-rs/loco/issues/41)
- adding tests for note requests [https://github.com/loco-rs/loco/pull/156](https://github.com/loco-rs/loco/pull/156)
- Define the min rust version the loco can run [https://github.com/loco-rs/loco/pull/164](https://github.com/loco-rs/loco/pull/164)
- Added `cargo loco doctor` cli command for validate and diagnose configurations. [https://github.com/loco-rs/loco/pull/145](https://github.com/loco-rs/loco/pull/145)
- Added ability to specify `settings:` in config files, which are available in context
- Adding compilation mode in the banner. [https://github.com/loco-rs/loco/pull/127](https://github.com/loco-rs/loco/pull/127)
- Support shuttle deployment generator. [https://github.com/loco-rs/loco/pull/124](https://github.com/loco-rs/loco/pull/124)
- Adding a static asset middleware which allows to serve static folder/data. Enable this section in config. [https://github.com/loco-rs/loco/pull/134](https://github.com/loco-rs/loco/pull/134)
  ```yaml
  static:
    enable: true
    # ensure that both the folder.path and fallback file path are existence.
    must_exist: true
    folder:
      uri: "/assets"
      path: "frontend/dist"
    fallback: "frontend/dist/index.html"
  ```
- fix: `loco generate request` test template. [https://github.com/loco-rs/loco/pull/133](https://github.com/loco-rs/loco/pull/133)
- Improve docker deployment generator. [https://github.com/loco-rs/loco/pull/131](https://github.com/loco-rs/loco/pull/131)

## v0.1.6

- refactor: local settings are now `<env>.local.yaml` and available for all environments, for example you can add a local `test.local.yaml` and `development.local.yaml`
- refactor: removed `config-rs` and now doing config loading by ourselves.
- fix: email template rendering will not escape URLs
- Config with variables: It is now possible to use [tera](https://keats.github.io/tera) templates in config YAML files

Example of pulling a port from environment:

```yaml
server:
  port: { { get_env(name="NODE_PORT", default=5150) } }
```

It is possible to use any `tera` templating constructs such as loops, conditionals, etc. inside YAML configuration files.

- Mailer: expose `stub` in non-test

- `Hooks::before_run` with a default blank implementation. You can now code some custom loading of resources or other things before the app runs
- an LLM inference example, text generation in Rust, using an API (`examples/inference`)
- Loco starters version & create release script [https://github.com/loco-rs/loco/pull/110](https://github.com/loco-rs/loco/pull/110)
- Configure Cors middleware [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
- `Hooks::after_routes` Invoke this function after the Loco routers have been constructed. This function enables you to configure custom Axum logics, such as layers, that are compatible with Axum. [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
- Adding docker deployment generator [https://github.com/loco-rs/loco/pull/119](https://github.com/loco-rs/loco/pull/119)

DOCS:

- Remove duplicated docs in auth section
- FAQ docs: [https://github.com/loco-rs/loco/pull/116](https://github.com/loco-rs/loco/pull/116)

ENHANCEMENTS:

- Remove unused libs: [https://github.com/loco-rs/loco/pull/106](https://github.com/loco-rs/loco/pull/106)
- turn off default features in tokio [https://github.com/loco-rs/loco/pull/118](https://github.com/loco-rs/loco/pull/118)

## 0.1.5

NEW FEATURES

- `format:html` [https://github.com/loco-rs/loco/issues/74](https://github.com/loco-rs/loco/issues/74)
- Create a stateless HTML starter [https://github.com/loco-rs/loco/pull/100](https://github.com/loco-rs/loco/pull/100)
- Added worker generator + adding a way to test workers [https://github.com/loco-rs/loco/pull/92](https://github.com/loco-rs/loco/pull/92)

ENHANCEMENTS:

- CI: allows cargo cli run on fork prs [https://github.com/loco-rs/loco/pull/96](https://github.com/loco-rs/loco/pull/96)
