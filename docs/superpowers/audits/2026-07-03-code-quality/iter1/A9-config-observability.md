# Area: A9 · Config, Errors & Observability

## Scope (files reviewed, with LOC)
- `src/config/mod.rs` (196) — `Config` struct, `from_folder`/`new` loading, `get_jwt_config`, `Display`
- `src/config/auth.rs` (54), `src/config/cache.rs` (33), `src/config/database.rs` (100),
  `src/config/logger.rs` (86), `src/config/mailer.rs` (96), `src/config/queue.rs` (98),
  `src/config/server.rs` (83) — 746 LOC total in `src/config/`
- `src/env_vars.rs` (31)
- `src/errors.rs` (177)
- `src/logger.rs` (224)
- `src/doctor.rs` (388)
- `src/depcheck.rs` (251)
- `src/data.rs` (163)
- `src/cargo_config.rs` (171)
- Cross-cutting context read (not separately scored, but load-bearing for "HTTP status mapping"
  per task brief): `src/controller/mod.rs:1-253` (the `impl IntoResponse for Error`),
  `src/environment.rs` (138, full read for env-var duplication check)

## Scores
| KPI | Score | One-line justification w/ primary cite |
|---|---|---|
| 1. Holistic vision | 5 | Backtrace design (`errors.rs:15-23`) is only half-wired: 2 of 3 `.bt()` call sites are commented out (`errors.rs:155,159`); `doctor.rs` reinvents ad hoc version parsing while `depcheck.rs`, its sibling file solving the identical "check a crate's version" problem, does it cleanly via a real crate (`depcheck.rs:10,54`) — same problem, two disjoint philosophies in the same area. |
| 2. Economy of concepts | 6 | Config module is well-split by domain (7 small files); but `env_vars.rs`'s entire stated purpose — "centralizes... a single location" (`env_vars.rs:1-4`) — is undermined by `environment.rs:22-24` re-declaring the identical `LOCO_ENV`/`RAILS_ENV`/`NODE_ENV` constants. |
| 3. Low LOC | 7 | Config structs are lean, mostly declarative serde types with sane `#[serde(default)]` use (e.g. `config/queue.rs:19,26,34...`); no needless verbosity found. |
| 4. Non-brittle | 3 | `Config::from_folder` picks **one** of `{env}.local.yaml` / `{env}.yaml` — first-existing-wins, **no merge** (`config/mod.rs:155-160`) — a `.local.yaml` must duplicate the *entire* config or deserialization fails; `logger.rs:196` `.expect()`s on a bad `override_filter` string turning a config typo into a boot-time panic; `doctor.rs:44,323` parse human CLI text with regex (see finding 3, 4). |
| 5. Maintainable (DDD) | 6 | Domain types own their behavior well (`SmtpMailer::tls_mode` at `config/mailer.rs:80-86`); but `Error` (`errors.rs:32-151`) is one flat 26+-variant enum spanning HTTP, DB, mail, worker, storage and cache concerns with no sub-grouping. |
| 6. Correctness | 3 | Zero `#[cfg(test)]` in `config/mod.rs`, any `config/*.rs`, `errors.rs`, `logger.rs`, or `doctor.rs` (verified via `grep -c "cfg(test)"`, all `0`) — the exact "config precedence" and "Error→HTTP mapping" logic the task calls out is entirely untested; plus the semver-truncation bug in `doctor.rs:323` (finding 4). |
| 7. No reinvented wheels | 5 | `logger.rs` is a fair, thin, justified wrapper over `tracing-subscriber` (not a reinvention — praised below). Config's own YAML+env layering is a deliberate, documented design (judged fairly, not penalized). But `doctor.rs:65-97` shells to `cargo search` and regexes free text instead of calling crates.io's JSON API; `cargo_config.rs:53-63` hand-rolls nested TOML `Option` chains. |
| **Overall** | **5** | Genuinely clean logger and config-struct design, offset by an unmerged config-override footgun, an untested Error/HTTP mapping, and hand-rolled, regex-based version-discovery in `doctor.rs` that is both more brittle and less tested than its sibling `depcheck.rs`. |

## Evidence log

1. **FACT**: `Config::from_folder` builds `files = [{env}.local.yaml, {env}.yaml]` and does
   `files.iter().find(|p| p.exists())` — the **first existing file wins entirely**; the other is
   never read or merged (`src/config/mod.rs:153-174`, esp. `:155-160`).
   **Judgment**: This is not "layered config" in the figment/config-crate sense the task brief
   invites comparison to — there is no field-level merge. A `development.local.yaml` must be a
   complete, valid `Config` (every non-`Option` field present) or `serde_yaml::from_str` fails
   whole-file. This is a real, silent usability trap: a dev who creates a `.local.yaml` meaning
   to override just `database.uri` will get a deserialization error instead. Corroborated
   independently by the repo's own prior audit doc, which flags this exact gap as
   "STALE/INCOMPLETE (G-C2)... the `.local.yaml` override tier is not documented"
   (`docs/superpowers/specs/2026-07-03-loco-1.0-inventory/05-auth-config-security.md:275`).
   **KPI**: 1 (holistic vision), 4 (non-brittle), 6 (correctness/tests — untested behavior).
   **Severity**: HIGH.

2. **FACT**: `env_vars.rs` declares `LOCO_ENV`, `RAILS_ENV`, `NODE_ENV` (`env_vars.rs:10-14`)
   with a module doc explicitly stating its purpose is to centralize env-var keys "from a single
   location in the codebase" (`env_vars.rs:1-4`). `environment.rs:22-24` re-declares the
   **identical three constants** with the identical string values, used by `environment.rs`'s own
   `resolve_from_env` (`environment.rs:33-38`, using the `env_vars::` versions) while its test
   module uses the *locally redeclared* ones (`environment.rs:109-121`, `unsafe { env::remove_var(LOCO_ENV) }`
   referring to the local const, not `env_vars::LOCO_ENV`).
   **Judgment**: Two independent, unlinked sources of truth for the same 3 keys, exactly the kind
   of thing `env_vars.rs` was written to prevent — if one is renamed the other silently drifts.
   **KPI**: 1, 2. **Severity**: MEDIUM.

3. **FACT**: `check_cratesio_version` (`doctor.rs:65-97`) shells out to `cargo search <crate> --limit 1`
   (`doctor.rs:67-70`) and extracts the version with the regex `(?m)^[^"]*"([^"]+)"`
   (`doctor.rs:44`) against `cargo search`'s free-text, human-oriented stdout (format:
   `name = "version"    # description`), not a stable machine contract.
   **Judgment**: This is parsing a CLI tool's display format with a regex as if it were an API.
   Any change to `cargo search`'s output formatting (already known to be an unstable/soft-deprecated
   command family) silently breaks this without a compile-time signal, and there is no test
   covering it (no test in `doctor.rs` invokes this function — confirmed 0 `cfg(test)` in
   `doctor.rs`). Compare to the crate's own `depcheck.rs`, in the same area, which reads a
   structured `Cargo.lock` via the `cargo-lock` crate and is fully unit-tested
   (`depcheck.rs:102-251`).
   **KPI**: 4 (non-brittle), 6 (correctness), 7 (reinvented wheel — crates.io JSON API exists).
   **Severity**: MEDIUM.

4. **FACT**: `check_seaorm_cli` (`doctor.rs:318-362`) extracts the CLI's reported version with
   `Regex::new(r"(\d+\.\d+\.\d+)")` (`doctor.rs:323`), which matches only the `X.Y.Z` numeric core
   and **drops any pre-release suffix** (e.g. `2.0.0-rc.3` → captured as `2.0.0`). It then compares
   via `semver::Version` (`doctor.rs:333,336,339`) against `MIN_SEAORMCLI_VER = "2.0.0-rc"`
   (`doctor.rs:39`). Because a stripped `2.0.0` outranks any `2.0.0-rc*` in semver precedence, an
   installed `sea-orm-cli 2.0.0-rc.1` (an actual pre-release, arguably below the intended bar) is
   silently reported as satisfying the minimum, since the regex already threw away the very
   suffix the comparison depends on.
   **Judgment**: A genuine logic bug in a version-gating check, sourced from the regex-based
   parsing approach itself — not a hypothetical, it's directly derivable from the two lines of
   code. No test exercises this path (0 tests in `doctor.rs`).
   **KPI**: 6 (correctness). **Severity**: MEDIUM (dev-tooling only, not a runtime/production path).

5. **FACT**: `impl IntoResponse for Error` (`controller/mod.rs:180-253`) gives distinct HTTP
   treatment to only 6 of the enum's 26+ variants (`NotFound`, `Unauthorized`, `CustomError`,
   `WithBacktrace`, `BadRequest`, `JsonRejection`, `Validation`); every other variant — including
   `TaskNotFound` (`errors.rs:48`), `QueueProviderMissing` (`errors.rs:46`), `Worker(String)`
   (`errors.rs:78-79`), `DB`, `Model`, `Storage`, `Cache`, `Scheduler`, and all IO/transport
   errors — falls through the wildcard arm to a flat `500 Internal Server Error`
   (`controller/mod.rs:245-248`). Additionally, `WithBacktrace` is unconditionally mapped to
   `400 Bad Request` regardless of the wrapped variant's real semantics
   (`controller/mod.rs:220-227`), and the doc comment atop `errors.rs` (`errors.rs:15-23`)
   presents `.bt()` as the general convention for adding backtraces to *any* error — meaning any
   future `SomeError.bt()` on, say, a `DB` error would surface to callers as 400, not 500.
   **Judgment**: Reasonable as a conservative default (don't leak internals), but it means
   `TaskNotFound` — whose name says "not found" — renders as a generic 500 rather than 404, and
   the backtrace-wrapping mechanism and the HTTP-mapping mechanism are not co-designed: one can
   silently invert the other's intended status code. This is exactly the "cohesion and
   completeness" the task asked to judge, and it is untested — no test in the crate exercises
   `IntoResponse for Error` across variants (checked `errors.rs`, `controller/mod.rs`; no
   `#[cfg(test)]` in either).
   **KPI**: 5 (maintainability/cohesion), 6 (correctness/completeness). **Severity**: MEDIUM.

6. **FACT**: `Error::wrap` and `Error::msg` (`errors.rs:154-160`) both exist to build an ad hoc
   error from `impl std::error::Error`, but with different information-preservation semantics:
   `wrap` boxes the original into `Self::Any(Box::new(err))` (preserves type/source chain),
   while `msg` throws it away into `Self::Message(err.to_string())` (string only). Both have a
   commented-out `.bt()` call (`//.bt()` at `errors.rs:155` and `:159`) — dead, half-finished
   code left in a small, actively-used file.
   **Judgment**: Two near-identical helpers with an undocumented, easy-to-misuse distinction,
   plus literal commented-out intent (`//.bt()`) is a clear patch-on-patch smell.
   **KPI**: 1, 2. **Severity**: LOW-MEDIUM.

## Patch-on-patch smells (specific, cited)
- Commented-out code: `errors.rs:155` (`//.bt()`) and `errors.rs:159` (`//.bt()`) — dead intent
  left inline in the constructor helpers.
- Stale `#[allow(dead_code)]`: `env_vars.rs:27` marks `get_or_default` dead, but it is actively
  used at `src/db/connect.rs:201` — the attribute is simply wrong/stale, meaning it was added
  when the function was unused and never removed once a caller appeared.
- Duplicated constants across module boundaries: `env_vars.rs:10-14` vs `environment.rs:22-24`
  (see finding 2).
- Two incompatible approaches to "check a crate's version" living in sibling files of the same
  area: `depcheck.rs` (structured `cargo-lock` parsing, tested) vs `doctor.rs`
  (`cargo search`/`sea-orm-cli --version` + regex, untested) — see findings 3–4.
- `#[allow(clippy::cognitive_complexity)]` on `IntoResponse for Error` (`controller/mod.rs:182`,
  cross-cutting context) — a complexity-suppression on the very function whose completeness this
  area's brief asks to assess; a symptom of the enum having grown past what one match cleanly
  handles.

## Library hypotheses
- **HYPOTHESIS**: `doctor.rs:65-97` (`check_cratesio_version`, subprocess + regex over
  `cargo search`) → replace with a direct HTTP GET to crates.io's public JSON API
  (`https://crates.io/api/v1/crates/{name}`, returning `.crate.max_version`) using whatever HTTP
  client Loco already links for mailer/HTTP concerns. **Why it might be simpler/cleaner**: no
  subprocess spawn, no fragile text-format regex, structured JSON parse, directly testable with a
  mocked response. **Risk / why it might not fit**: adds a network round-trip dependency and
  requires picking/adding an async or blocking HTTP client if one isn't already trivially
  reachable from this sync function; crates.io's terms/rate limits for the raw API need checking
  for a CLI-invoked tool run frequently in CI. **NEEDS SPIKE.**
- **HYPOTHESIS**: `cargo_config.rs:19-63` (hand-rolled `toml::Table` chained `.and_then()` nested
  lookups) → the `cargo_metadata` crate (`cargo metadata --format-version=1` + typed JSON) for a
  more robust view of `package.metadata` sections. **Why it might be simpler**: typed access to
  metadata tables, avoids re-implementing "read Cargo.toml, walk 4 keys" by hand. **Risk / why it
  might not fit**: `cargo_metadata` shells out to `cargo metadata`, which is slower and heavier
  than a direct file read for a single small metadata lookup, and the current code is already
  quite short (only 14 lines of table-walking, `cargo_config.rs:53-63`) — this is a weak
  hypothesis, likely NOT worth the dependency per the rubric's "don't recommend a swap that adds
  weight for marginal gain." **NEEDS SPIKE**, low priority.
- **NOT a reinvention (fair-judged, not a finding)**: `config/mod.rs`'s YAML+Tera+env-var loading
  is a deliberate, documented layering strategy distinct from `figment`/`config`-crate merging.
  Per the task brief this is judged on its own terms, not penalized for not being `figment` — the
  actual defect found (finding 1) is the *lack of merge semantics*, not the choice to avoid the
  crate.

## What is genuinely excellent (cited — be specific)
- `logger.rs:104-224` is a clean, thin wrapper over `tracing-subscriber`: three well-named
  functions (`init`, `init_env_filter`, `init_layer`), one static `OnceLock` for the worker guard
  (`logger.rs:82`), and a clearly documented filter-precedence policy in the doc comment
  (`logger.rs:85-99`) that is faithfully implemented by `init_env_filter` (`logger.rs:177-197`).
  This is exactly the "thin wrapper, not reinvention" the task brief hoped to find — no
  hand-rolled level parsing, delegates entirely to `EnvFilter`.
- `errors.rs:24-28,166-177` — the backtrace-on-demand design (`bt()` only materializes a
  `std::backtrace::Backtrace` when `RUST_BACKTRACE` makes it non-disabled/non-unsupported) is a
  genuinely clever, low-cost mechanism; the intent is well-explained in the header comment
  (`errors.rs:15-23`).
- `depcheck.rs` as a whole: focused single responsibility, delegates real parsing to the
  `cargo-lock` crate (`depcheck.rs:10,54`), and is the best-tested file in the area (5 tests
  covering ok/invalid/not-found/exact-match/empty-map cases, `depcheck.rs:114-251`).
- `config/mailer.rs:76-86` — `SmtpMailer::tls_mode()` cleanly resolves a backwards-compatible
  `secure: bool` flag against a new explicit `tls: Option<MailerTls>` field, with the precedence
  rule documented directly on the field (`config/mailer.rs:62-69`) — a good example of handling
  config evolution without breaking existing YAML.
- `cargo_config.rs` and `depcheck.rs` both carry a clear division-of-responsibility comment
  pointing at each other (`cargo_config.rs:7-8`) — a rare, welcome bit of architectural
  self-documentation in this area.

## Top 3 things that would most raise the area's quality
1. Make `Config::from_folder` actually merge `{env}.yaml` as a base with `{env}.local.yaml` as an
   overlay (even a simple `serde_yaml::Value` deep-merge), or explicitly document/rename the
   current "whole-file override" behavior so it stops being a silent trap — this is the single
   highest-leverage fix (`config/mod.rs:153-174`).
   
2. Add unit tests for `errors.rs`'s `Error → HTTP status` mapping (`controller/mod.rs:180-253`)
   and for `Config::from_folder` precedence (`config/mod.rs:153-174`) — currently the two most
   "load-bearing" pieces of this area (per the task brief itself) have zero test coverage.

3. Replace `doctor.rs`'s `cargo search`-and-regex / `sea-orm-cli --version`-and-regex version
   checks with structured parsing (crates.io JSON API and/or `semver`-safe extraction that
   doesn't truncate pre-release tags), fixing the pre-release-truncation bug (`doctor.rs:323`)
   and bringing `doctor.rs` up to the same rigor as its sibling `depcheck.rs`.
