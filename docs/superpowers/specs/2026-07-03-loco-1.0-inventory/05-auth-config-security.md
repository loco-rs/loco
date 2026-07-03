# Inventory 05 — Auth, Configuration, Security primitives

Verified against code on branch `release/0.17.0`. All `file:line` refer to the actual repo. "Only VERIFIED docs" — every claim below is checked against source.

---

## PART A — AUTHENTICATION

### A1. JWT core (`loco_rs::auth::jwt`)
**Purpose:** Generate/validate HS-family JWTs carrying arbitrary flattened claims.

- Module gate: `src/auth/mod.rs:1-2` — `jwt` module is `#[cfg(feature = "auth_jwt")]`. So `loco_rs::auth::jwt` **only exists when `auth_jwt` is enabled** (it is a default feature).
- Public API surface (`src/auth/jwt.rs`):
  - `struct UserClaims { pub pid: String, exp: u64 (private), pub claims: Map<String,Value> }` — `jwt.rs:18-23`. `exp` is **private** (not user-settable/readable directly). Custom claims are `#[serde(flatten)]` so they serialize at the top level alongside `pid`/`exp`.
  - `struct JWT { secret, algorithm }` — `jwt.rs:33-37`.
  - `JWT::new(secret: &str) -> Self` — `jwt.rs:42`. Constructs with default algorithm.
  - `JWT::algorithm(self, algorithm: Algorithm) -> Self` — `jwt.rs:51` (builder to override algorithm).
  - `JWT::generate_token(&self, expiration: u64, pid: String, claims: Map<String,Value>) -> JWTResult<String>` — `jwt.rs:70-87`. `exp = now + expiration` via `saturating_add`. **Encodes with `EncodingKey::from_base64_secret` — the configured secret MUST be valid base64.**
  - `JWT::validate(&self, token: &str) -> JWTResult<TokenData<UserClaims>>` — `jwt.rs:102-111`. Uses `DecodingKey::from_base64_secret`; sets `validate.leeway = 0` (no clock-skew tolerance on expiry).
- Default algorithm: `const JWT_ALGORITHM: Algorithm = Algorithm::HS512` — `jwt.rs:13`. (Doc-worthy: default is **HS512**, not HS256.)
- Errors: returns `jsonwebtoken::errors::Result` directly (not Loco's `Error`).

### A2. Axum auth extractors (`loco_rs::controller::extractor::auth`)
**Purpose:** Drop-in `FromRequestParts` extractors that authenticate a request and hand you claims / the user model.

File: `src/controller/extractor/auth.rs`. Public items users touch:
- `struct JWT { pub claims: UserClaims }` — `auth.rs:104-107`. Extractor available **without** `with-db`. Validates token, returns claims only. Access user id via `auth.claims.pid`.
- `struct JWTWithUser<T: Authenticable> { pub claims: UserClaims, pub user: T }` — `auth.rs:50-55`, `#[cfg(feature = "with-db")]`. Validates token AND loads the user via `T::find_by_claims_key(db, claims.pid)`.
- `struct ApiToken<T: Authenticable> { pub user: T }` — `auth.rs:254-259`, `#[cfg(feature = "with-db")]`. Reads a `Bearer <key>` header and loads user via `T::find_by_api_key(db, key)`. **Note: API key is read only from the `Authorization: Bearer` header — the `location` config does NOT apply to `ApiToken`, only to JWT.**
- Free functions (public, reusable):
  - `extract_jwt_from_request_parts<S>(&Parts, &S) -> Result<JWT, Error>` — `auth.rs:126`. Non-mutating variant for use outside the extractor machinery.
  - `get_jwt_from_config(&AppContext) -> Result<&config::JWT>` — `auth.rs:152`. Errors "auth not configured" / "JWT token not configured".
  - `extract_token(&config::JWT, &Parts) -> Result<String>` — `auth.rs:167`. Tries each configured location in order.
  - `extract_token_from_header(&HeaderMap)`, `extract_token_from_cookie(name, &Parts)`, `extract_token_from_query(name, &Parts)` — `auth.rs:208 / 224 / 239`.
- Constants: `TOKEN_PREFIX = "Bearer "`, `AUTH_HEADER = "authorization"` — `auth.rs:45-46`.
- Rejection type is Loco `Error`; failures map to `Error::Unauthorized(..)` (401) or `Error::InternalServerError` (500 on DB error) — `auth.rs:79-97`, `284-292`.

### A3. `Authenticable` trait (contract the user model must implement)
- `src/model/mod.rs:57-59`: `pub trait Authenticable: Clone { async fn find_by_api_key(db, api_key) -> ModelResult<Self>; async fn find_by_claims_key(db, claims_key) -> ModelResult<Self>; }`
- Required for `JWTWithUser` and `ApiToken`. (JWT-only extractor does not need it.)

### A4. JWT token location resolution
- `get_jwt_locations(Option<&JWTLocationConfig>)` — `auth.rs:181-189`. **Default when unset = `Bearer`.** Single/Multiple handled; Multiple tried in order until one yields a token.
- Config types in `src/config/auth.rs`:
  - `enum JWTLocation` (`#[serde(tag="from")]`) — `auth.rs:37-44`: `Bearer` | `Query { name }` | `Cookie { name }`.
  - `enum JWTLocationConfig` (`#[serde(untagged)]`) — `auth.rs:49-54`: `Single(JWTLocation)` | `Multiple(Vec<JWTLocation>)`.

### A5. Password hashing / random tokens (`loco_rs::hash`)
File: `src/hash.rs`. **Purpose:** Argon2id password hashing + random token generation.
- `hash_password(pass: &str) -> Result<String>` — `hash.rs:21-33`. Argon2id, `Version::V0x13`, `Params::default()`, random salt via `OsRng`. Returns Loco `Result`; maps failures via `Error::msg` (see 1.0 note E1).
- `verify_password(pass: &str, hashed_password: &str) -> bool` — `hash.rs:48-58`. `#[must_use]`. Returns `false` on any parse/verify failure (no error surfaced).
- `random_string(length: usize) -> String` — `hash.rs:72-78`. Alphanumeric, used for api keys / reset tokens in the starter.

---

## PART B — CONFIGURATION (CANONICAL REFERENCE)

Recently split from a single `config.rs` into `src/config/{mod,auth,cache,database,logger,mailer,queue,server}.rs`. All sub-structs are re-exported flat from `loco_rs::config` (`src/config/mod.rs:43-49`), so users reference e.g. `config::Database`, `config::JWT`.

### B0. Loading & precedence
- `Config::new(env) -> Result<Self>` — `mod.rs:128`. Loads from default folder `config/` (`mod.rs:55-57`).
- `Config::from_folder(env, path) -> Result<Self>` — `mod.rs:153-174`. **File precedence (first existing wins):** `{env}.local.yaml` then `{env}.yaml` (`mod.rs:155-158`). No file found → `Error::Message`.
- Every YAML file is rendered as a **Tera template** before parse — `mod.rs:170` calls `crate::tera::render_string`. `src/tera.rs:5-8` uses `Tera::one_off(.., autoescape=false)`. **`get_env(name=.., default=..)` is Tera's built-in function** (NOT custom-registered by Loco) — used in the shipped YAML docstrings (`database.rs:12`, `server.rs:11`, `mailer.rs:27`).
- Parse errors → `Error::YAMLFile(err, path)` — `mod.rs:172-173`.
- Accessor: `Config::get_jwt_config(&self) -> Result<&JWT>` — `mod.rs:180-188`. `Display` for `Config` dumps YAML — `mod.rs:191-196`.
- Environment resolution (`src/environment.rs`): `resolve_from_env()` precedence = `LOCO_ENV` → `RAILS_ENV` → `NODE_ENV` → default `"development"` (`environment.rs:33-37`, `DEFAULT_ENVIRONMENT = "development"` line 21). Config-folder override env var = `LOCO_CONFIG_FOLDER` (see D2).

### B1. Top-level `Config` struct — `mod.rs:62-92`
| YAML key | Type | Required? | Notes (file:line) |
|---|---|---|---|
| `logger` | `Logger` | required | `mod.rs:64` |
| `server` | `Server` | required | `mod.rs:65` |
| `database` | `Database` | required **iff** `with-db` feature | `#[cfg(feature="with-db")]` `mod.rs:66-67` |
| `cache` | `CacheConfig` | optional, `#[serde(default)]` → `Null` | `mod.rs:68-69` |
| `queue` | `Option<QueueConfig>` | optional | `mod.rs:70` |
| `auth` | `Option<Auth>` | optional | `mod.rs:71` |
| `workers` | `Workers` | optional, `#[serde(default)]` | `mod.rs:72-73` |
| `mailer` | `Option<Mailer>` | optional | `mod.rs:74` |
| `initializers` | `Option<Initializers>` (`BTreeMap<String, Value>`) | optional | `mod.rs:75`, type at `mod.rs:106` |
| `settings` | `Option<serde_json::Value>` | optional, `#[serde(default)]` | `mod.rs:88-89`; surfaces at `ctx.config.settings` |
| `scheduler` | `Option<scheduler::Config>` | optional | `mod.rs:91` (owned by scheduler area) |

### B2. `auth:` → `Auth` — `src/config/auth.rs`
```yaml
auth:
  jwt:                       # Auth.jwt: Option<JWT>  (auth.rs:16)
    location:                # Option<JWTLocationConfig>, default Bearer (auth.rs:24)
      from: Bearer           # or Query {from: Query, name: ...} / Cookie {from: Cookie, name: ...}
    secret: <base64 secret>  # String, REQUIRED (auth.rs:26) — must be valid base64
    expiration: 604800       # u64 seconds, REQUIRED (auth.rs:28)
```
- `location` single form is a map with `from:` tag; multiple form is a YAML list of such maps (untagged enum, `auth.rs:36-54`).
- **Discrepancy vs jwt.rs:** `secret` is passed to `from_base64_secret` — docs never state it must be base64 (see gap G1).

### B3. `server:` → `Server` / `Workers` — `src/config/server.rs`
```yaml
server:
  binding: localhost     # String, #[serde(default)] "localhost"  (server.rs:33-34,47-49)
  port: 5150             # i32, REQUIRED  (server.rs:36)
  host: http://localhost # String, REQUIRED  (server.rs:38)
  ident: <string>        # Option<String>, "Server" header  (server.rs:40)
  middlewares: {...}      # middleware::Config, #[serde(default)]  (server.rs:44) — owned by middleware area
```
- `Server::full_url() -> "{host}:{port}"` — `server.rs:52-55`.
```yaml
workers:
  mode: BackgroundQueue  # WorkerMode enum, default BackgroundQueue  (server.rs:64-83)
```
- `WorkerMode`: `BackgroundQueue` (default, needs Redis/queue) | `ForegroundBlocking` | `BackgroundAsync` — `server.rs:71-83`.

### B4. `database:` → `Database` — `src/config/database.rs:22-84` (only when `with-db`)
```yaml
database:
  uri: <conn string>         # String, REQUIRED (postgres/sqlite examples in docstring)
  enable_logging: true       # bool, REQUIRED (SQLx statement logging)
  min_connections: 1         # u32, REQUIRED
  max_connections: 1         # u32, REQUIRED
  connect_timeout: 500       # u64 ms, REQUIRED
  idle_timeout: 500          # u64 ms, REQUIRED
  acquire_timeout: <ms>      # Option<u64>
  auto_migrate: false        # bool, #[serde(default)]
  dangerously_truncate: false# bool, #[serde(default)]
  dangerously_recreate: false# bool, #[serde(default)]
  run_on_start: <sql string> # Option<String>; SQLite PRAGMA defaults listed in docstring (database.rs:66-83)
```
- Default helper fns (used by queue configs, not by Database itself): `db_min_conn()=1`, `db_max_conn()=20`, `db_connect_timeout()=500`, `db_idle_timeout()=500` — `database.rs:86-100`. **Note:** `Database`'s own numeric fields are REQUIRED (no serde default); the defaults only feed queue configs (B7).

### B5. `logger:` → `Logger` — `src/config/logger.rs:21-86`
```yaml
logger:
  enable: true            # bool, REQUIRED
  pretty_backtrace: false # bool, #[serde(default)] — forces RUST_BACKTRACE=1 when true
  level: debug            # logger::LogLevel, REQUIRED — trace|debug|info|warn|error
  format: compact         # logger::Format, REQUIRED — compact|pretty|json
  override_filter: <str>  # Option<String> — EnvFilter directive
  file_appender:          # Option<LoggerFileAppender>
    enable: true          # bool, REQUIRED within block
    non_blocking: false   # bool, #[serde(default)]
    level: info           # LogLevel, REQUIRED within block
    format: json          # Format, REQUIRED within block
    rotation: daily       # logger::Rotation, REQUIRED within block
    dir: ./logs           # Option<String> (default ./logs)
    filename_prefix: <s>  # Option<String>
    filename_suffix: <s>  # Option<String>
    max_log_files: 7      # usize, REQUIRED within block
```
`LogLevel`/`Format`/`Rotation` enums live in `src/logger.rs` (logger area owns exact variants).

### B6. `mailer:` → `Mailer` — `src/config/mailer.rs:29-96`
```yaml
mailer:
  stub: false          # bool, #[serde(default)] — capture instead of send
  smtp:                # Option<SmtpMailer>
    enable: true       # bool, REQUIRED
    host: localhost    # String, REQUIRED
    port: 1025         # u16, REQUIRED
    secure: false      # bool, REQUIRED — legacy shorthand: true=STARTTLS, false=cleartext
    tls: implicit      # Option<MailerTls> #[serde(default)] — starttls|implicit|none; OVERRIDES `secure`
    auth:              # Option<MailerAuth>
      user: <string>   # String, REQUIRED in block
      password: <str>  # String, REQUIRED in block
    hello_name: <str>  # Option<String> — EHLO client id
```
- `MailerTls` (`#[serde(rename_all="lowercase")]`): `starttls` | `implicit` | `none` — `mailer.rs:38-50`.
- `SmtpMailer::tls_mode()` resolves effective mode: explicit `tls` wins, else `secure` → Starttls/None — `mailer.rs:76-87`. (1.0-era addition: implicit TLS / port 465 support; docs currently only show `secure`.)

### B7. `queue:` → `QueueConfig` — `src/config/queue.rs:5-98`
`#[serde(tag = "kind")]` enum: `Redis` | `Postgres` | `Sqlite` — `queue.rs:6-14`.
```yaml
# kind: Redis  (RedisQueueConfig, queue.rs:16-28)
queue:
  kind: Redis
  uri: redis://...          # String, REQUIRED
  dangerously_flush: false  # bool, #[serde(default)]
  queues: [high, low]       # Option<Vec<String>> — priority order, first = most important
  num_workers: 2            # u32, default 2 (num_workers())
# kind: Postgres  (PostgresQueueConfig, queue.rs:30-57)
  kind: Postgres
  uri: postgres://...       # String, REQUIRED
  dangerously_flush: false  # bool, default
  enable_logging: false     # bool, default
  max_connections: 20       # u32, default db_max_conn()=20
  min_connections: 1        # u32, default db_min_conn()=1
  connect_timeout: 500      # u64, default db_connect_timeout()=500
  idle_timeout: 500         # u64, default db_idle_timeout()=500
  poll_interval_sec: 1      # u32, default pgq_poll_interval()=1
  num_workers: 2            # u32, default 2
# kind: Sqlite  (SqliteQueueConfig, queue.rs:59-86) — same fields as Postgres, poll default sqlt_poll_interval()=1
```

### B8. `cache:` → `CacheConfig` — `src/config/cache.rs:4-33`
`#[serde(tag = "kind")]` enum, **default = `Null`** (`cache.rs:14-15`):
```yaml
cache:
  kind: InMem              # InMemCacheConfig — requires feature cache_inmem
  max_capacity: 33554432   # u64, default 32*1024*1024 (cache.rs:20-26)
# --- or ---
  kind: Redis              # RedisCacheConfig — requires feature cache_redis
  uri: redis://...         # String, REQUIRED
  max_size: <u32>          # u32, REQUIRED — pool max connections
# --- or ---
  kind: Null               # default; no-op cache
```
- `InMem`/`Redis` variants are **feature-gated** (`cache_inmem` / `cache_redis`); if the feature is off, that YAML kind won't deserialize.

---

## PART C — FEATURE FLAGS & DEPENDENCIES (Cargo.toml)

- Default features (`Cargo.toml:28-36`): `auth_jwt`, `cli`, `with-db`, `local` (assumed), `bg_sqlt` + others.
- `auth_jwt = ["dep:jsonwebtoken", "jsonwebtoken/rust_crypto"]` — `Cargo.toml:41`. **1.0 note:** comment at `Cargo.toml:37-40` documents that jsonwebtoken 10 no longer bundles a crypto backend, so Loco explicitly selects the pure-Rust `rust_crypto` backend so `auth_jwt` stays self-contained (no C toolchain / OpenSSL).
- `jsonwebtoken = { version = "10.3.0", optional = true, default-features = false }` — `Cargo.toml:124`.
- `argon2 = { version = "0.5", features = ["std"] }` — `Cargo.toml:122` (always on; `hash` module is not feature-gated).
- `rand = { version = "0.9", ... }` — `Cargo.toml:123`.
- Cache/queue feature gates referenced by config: `cache_inmem`, `cache_redis`, `bg_redis`/`bg_pg`/`bg_sqlt`.
- **Gate summary:** `auth::jwt` module + `auth_jwt` feature control JWT *generation/validation*; the axum extractors (`controller::extractor::auth`) are NOT behind `auth_jwt` — but `JWTWithUser`/`ApiToken` are behind `with-db`; the plain `JWT` extractor needs neither.

---

## PART D — SECRETS / ENV HANDLING

### D1. `src/env_vars.rs` — centralized env keys
- Helpers: `get(key) -> Result<String, VarError>` (`env_vars.rs:23`), `get_or_default(key, default) -> String` (`env_vars.rs:29`, `#[allow(dead_code)]`).
- Constants: `POSTGRES_DB_OPTIONS="LOCO_POSTGRES_DB_OPTIONS"` (`with-db` gated), `LOCO_ENV`, `RAILS_ENV`, `NODE_ENV`, `CONFIG_FOLDER="LOCO_CONFIG_FOLDER"`, `SCHEDULER_CONFIG="SCHEDULER_CONFIG"`, `LOCO_DATA_FOLDER_ENV="LOCO_DATA"` — `env_vars.rs:8-20`.

### D2. Full env-var surface relevant to this area
| Env var | Purpose | Source |
|---|---|---|
| `LOCO_ENV` / `RAILS_ENV` / `NODE_ENV` | select environment (precedence in that order) | `environment.rs:33-37` |
| `LOCO_CONFIG_FOLDER` | override `config/` folder | `env_vars.rs:16` (`CONFIG_FOLDER`) |
| `LOCO_DATA` | data folder path | `env_vars.rs:20` |
| `LOCO_POSTGRES_DB_OPTIONS` | extra pg options | `env_vars.rs:8` |
| `SCHEDULER_CONFIG` | scheduler config path | `env_vars.rs:18` |
| `RUST_BACKTRACE` | set to `1` by `logger.pretty_backtrace=true` | logger area / `your-project.md:311` |
| any (`get_env(name=..)`) | injected into YAML via Tera built-in | Tera, not Loco code |

- **Secrets pattern:** secrets (JWT secret, SMTP password, DB uri) are injected into YAML via Tera `get_env(...)`, resolved at `Config::from_folder` render time. No secret is hardcoded in framework code. There is no dedicated secrets vault/type — a secret is just a `String` field.

---

## PART E — 1.0-RELEVANT NOTES (verified)

- **E0. jsonwebtoken 10 / `rust_crypto`:** confirmed at `Cargo.toml:37-41,124`. Migration-relevant: enabling `auth_jwt` with `default-features=false` still builds; no C toolchain. Algorithm default remains HS512.
- **E1. Error-enum narrowing — VERIFIED.** `src/errors.rs:30-151` (the full `#[non_exhaustive] enum Error`) contains **NO `EnvVar` and NO `Hash` variant.** Confirmation of downstream effects:
  - `hash.rs` now surfaces hashing failures via `Error::msg(...)` → `Error::Message` (`hash.rs:31`) rather than a dedicated `Hash` variant.
  - env-var errors are not modeled in `Error`; `env_vars::get` returns raw `std::env::VarError`.
  - YAML parse still has `YAMLFile`/`YAML` variants (`errors.rs:66-70`); auth failures use `Unauthorized(String)` (`errors.rs:93`). Anyone matching on removed `Error::EnvVar`/`Error::Hash` must migrate. (Enum is `#[non_exhaustive]`.)

---

## PART F — GENERATOR / CLI SUPPORT

- **No auth code is generated by `loco-gen`.** Verified: `loco-gen/src/templates/` has `scheduler, mailer, controller, deployment, task, model, worker, scaffold, data, migration` — no `auth`/`user` template; grep for `jwt|auth|hash_password|Authenticable` in `loco-gen/src` returns nothing.
- Auth (users model, `/api/auth/*` controllers, mailer templates, `Authenticable` impl) ships in the **SaaS starter template** (`loco new` → "SaaS app (with DB and user auth)"). Starter templates are **not vendored in this repo** (no `starters/` dir). Doc references to generated routes (`/api/auth/register|login|forgot|reset|verify|current`) describe the starter, not framework code — they can only be verified against the external starter repo, not here.
- CLI: `-b` flag maps to `server.binding` (`your-project.md:301`). `cargo loco routes`, `cargo loco doctor` referenced in docs.

---

## PART G — DOC COVERAGE ASSESSMENT

Existing docs: `docs-site/content/docs/extras/authentication.md` (257 lines) and the config section embedded in `the-app/your-project.md` (~lines 194-314). **There is NO dedicated config-reference page and NO testing page.**

### authentication.md — rating: THIN / partially STALE
- ACCURATE: register/login/verify/forgot/reset flow narrative; `auth::JWT` and `auth::ApiToken<T>` extractor usage examples (match `auth.rs` API); `auth.claims.pid` access is correct.
- **STALE (G-A1):** `authentication.md:24` says "The `auth` feature comes as a default with the library. If desired, you can turn it off." — the actual feature flag name is `auth_jwt` (`Cargo.toml:41`); doc never names it. Should say `auth_jwt`.
- **MISSING (G-A2):** No mention of JWT token **location** config (`Bearer`/`Query`/`Cookie`, single vs multiple) — a whole config surface (`config/auth.rs:36-54`, `auth.rs:181-201`) is undocumented.
- **MISSING (G-A3):** No mention that default algorithm is **HS512** (`jwt.rs:13`) or that the secret must be **base64** (`jwt.rs:83,108`). Users hitting `from_base64_secret` errors have no guidance.
- **MISSING (G-A4):** `JWTWithUser<T>` extractor (`auth.rs:52`) is undocumented — only `JWT` and `ApiToken` shown.
- **MISSING (G-A5):** The `Authenticable` trait contract (`model/mod.rs:57-59`) — required for `ApiToken`/`JWTWithUser` — is not documented.
- **MISSING (G-A6):** `loco_rs::hash` (`hash_password`/`verify_password`/`random_string`) not documented anywhere in this area.

### your-project.md config section — rating: THIN → escalate to MISSING for a real reference
- ACCURATE: settings/custom-settings example; `LOCO_CONFIG_FOLDER` override (`your-project.md:214`); Tera placeholder/`get_env` explanation (`:216-228`); server `binding`/`host`/`port` prose (`:299-303`); logger `pretty_backtrace` note (`:311`).
- **MISSING (G-C1) — biggest gap:** No enumerated config reference. It defers everything to the rustdoc link (`your-project.md:314`). The `database`, `mailer`, `queue`, `cache`, `workers`, `auth` sections have **zero YAML-key documentation** on the site. This inventory's Part B is the canonical replacement.
- **STALE/INCOMPLETE (G-C2):** File precedence is `{env}.local.yaml` then `{env}.yaml` (`mod.rs:155-158`) — the `.local.yaml` override tier is not documented.
- **MISSING (G-C3):** Environment resolution precedence `LOCO_ENV`→`RAILS_ENV`→`NODE_ENV`→`development` (`environment.rs:33-37`) is not documented.
- **MISSING (G-C4):** Mailer `tls: implicit` / port-465 support (`mailer.rs:38-87`) is new and undocumented; docs would only ever show `secure`.
- **MISSING (G-C5):** No documentation of the config-module split (single `config.rs` → `src/config/*`) for anyone importing `config::Database` etc. — re-exports are flat (`mod.rs:43-49`) so imports are unaffected, worth a one-line reassurance in the migration guide.

### G-D. Overall
- The single most valuable 1.0 deliverable in this area: **a dedicated Config Reference page** built from Part B (exhaustive, verified). Second: **auth page rewrite** covering feature flag name, HS512+base64 secret, JWT locations, all three extractors, `Authenticable`, and `hash`.
- Constraint honored: every YAML key above is backed by a struct field with a cited `file:line`; nothing inferred from external starters is presented as framework fact.
