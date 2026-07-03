# Loco I/O Infrastructure Inventory (mailers · storage · cache)

Verified against source at branch `release/0.17.0`. All `file:line` refer to the actual repo.
Scope: `src/mailer/`, `src/storage/`, `src/cache/`. Config in `src/config/{mailer,cache}.rs`. Feature flags in `Cargo.toml`.

---

## 1. MAILER

### 1.1 Purpose
Background-delivered transactional email. Emails are enqueued as jobs and delivered by a `MailerWorker` over SMTP (lettre). Templates are Tera-rendered from embedded dirs.

### 1.2 Public API surface
- `trait Mailer` — `src/mailer/mod.rs:90`
  - `fn opts() -> MailerOpts` (default impl) — `src/mailer/mod.rs:93`
  - `async fn mail(ctx: &AppContext, email: &Email) -> Result<()>` — `src/mailer/mod.rs:101` (enqueues via `MailerWorker::perform_later_with_priority`)
  - `async fn mail_template(ctx, dir: &Dir, args: Args) -> Result<()>` — `src/mailer/mod.rs:114`
- `struct Email` — `src/mailer/mod.rs:46` (fields: from, to, reply_to, subject, text, html, bcc, cc, headers)
- `struct Args` — `src/mailer/mod.rs:34` (from, to, reply_to, locals, bcc, cc, headers)
- `struct EmailHeaders` — `src/mailer/mod.rs:25` (references, in_reply_to, message_id)
- `struct MailerOpts` — `src/mailer/mod.rs:70` (from, reply_to, priority)
- `struct MailerWorker` + `BackgroundWorker<Email>` impl — `src/mailer/mod.rs:137,143`; queue name = `"mailer"` (`:144`)
- `EmailSender` (`pub use`) — `src/mailer/mod.rs:9`, defined `src/mailer/email_sender.rs:28`
  - `EmailSender::smtp(&config::SmtpMailer) -> Result<Self>` — `email_sender.rs:46`
  - `EmailSender::stub() -> Self` — `email_sender.rs:89`
  - `EmailSender::deliveries() -> Deliveries` (feature `testing` only) — `email_sender.rs:97`
  - `async fn mail(&self, &Email)` — `email_sender.rs:118`
  - `enum EmailTransport { Smtp, Test }` — `email_sender.rs:18`
  - `struct Deliveries { count, messages }` (feature `testing`) — `email_sender.rs:34`
- `Template<'a>` / `Content` — `src/mailer/template.rs:50,42`; `Template::new(dir)` `:57`, `render(&locals)` `:63`
- Constants: `DEFAULT_FROM_SENDER = "System <system@example.com>"` `mod.rs:18`; `DEFAULT_MAILER_PRIORITY = 100` `mod.rs:22`

### 1.3 Config knobs (`src/config/mailer.rs`)
- `Mailer { smtp: Option<SmtpMailer>, stub: bool }` — `:30` (`stub` default false, `#[serde(default)]`)
- `SmtpMailer { enable, host, port: u16, secure: bool, tls: Option<MailerTls>, auth: Option<MailerAuth>, hello_name: Option<String> }` — `:54`
- `enum MailerTls { Starttls, Implicit, None }` (serde lowercase) — `:40`
- `SmtpMailer::tls_mode()` — `:80`. Precedence: explicit `tls` wins; else legacy `secure` → `secure:true`=Starttls(587), `secure:false`=None.
- `MailerAuth { user, password }` — `:91`
- YAML keys: `mailer.smtp.{enable,host,port,secure,tls,hello_name}`, `mailer.smtp.auth.{user,password}`, `mailer.stub`.
- **NEW in 0.17**: `tls` (implicit TLS / SMTPS port 465) and `hello_name` (EHLO ClientId). Previously `secure:true` could ONLY do STARTTLS — implicit TLS on 465 was impossible (see test `email_sender.rs:184-213`).

### 1.4 Template mechanics (`src/mailer/template.rs`)
- Requires 3 embedded files per mailer dir: `subject.t`, `html.t`, `text.t` (`:22-26`). Missing file → `Error::Message("no mailer template file found ...")` (`:33`). Rendered via `tera::render_string`.

### 1.5 Feature flags
- No dedicated feature flag; mailer is always compiled. `deliveries()` / `Deliveries` gated on `feature = "testing"`.

### 1.6 Generator / CLI
- `cargo loco generate mailer <name>` — dispatched `loco-gen/src/lib.rs:365` (`Component::Mailer`), template `loco-gen/src/templates/mailer/mailer.t`.
- Generates `src/mailers/<name>.rs` with a `send_welcome(ctx, to, msg)` sample (template passes `message` + `domain` locals — NOTE: differs from docs which show `name`), plus `welcome/{subject,html,text}.t`, and appends `pub mod <name>;` to `mailers/mod.rs`.

### 1.7 Doc coverage — `docs-site/content/docs/processing/mailers.md` → **STALE**
Discrepancies vs code:
1. **Missing `tls` / implicit-TLS (465) entirely.** Docs only show `secure: true/false` — the single most important 0.17 mailer change is undocumented. The sendgrid example uses port 587 + `secure: true`; no mention that 465/SMTPS needs `tls: implicit`.
2. **Missing `hello_name`** (EHLO client id) config field.
3. **Missing `bcc`, `cc`, and custom `headers`/`EmailHeaders`** (references/in_reply_to/message_id) on `Email`/`Args` — all real, tested (`email_sender.rs:252`), undocumented. Relevant for threading/notifications.
4. **Missing `priority` / `DEFAULT_MAILER_PRIORITY`** in `MailerOpts` — mailer jobs enqueue with priority 100; not documented.
5. Generator sample in docs (`locals: {name}`) does not match actual generated template (`{message, domain}`) — minor drift.
6. Testing section is ACCURATE (`deliveries()`, `stub: true`).

---

## 2. STORAGE

### 2.1 Purpose
Generic multi-driver file storage abstraction over Apache OpenDAL, with pluggable strategies (single / mirror / backup) and streaming support. Default is a `Null` driver (errors on use). Wired via `after_context` hook into `AppContext.storage: Arc<Storage>` (`src/app.rs:268`).

### 2.2 Core API — `src/storage/mod.rs`
- `struct Storage { stores: BTreeMap<String, Box<dyn StoreDriver>>, strategy: Box<dyn StorageStrategy> }` — `:53`
- `Storage::single(store) -> Self` — `:69` (default key `"store"`, `SingleStrategy`)
- `Storage::new(stores, strategy) -> Self` — `:79`
- Ops (each has a `_with_strategy`/`_with_policy` variant taking an explicit strategy):
  - `upload(path, &Bytes)` `:108` / `upload_with_strategy` `:123`
  - `download::<T: TryFrom<Contents>>(path)` `:156` / `download_with_policy` `:170`
  - `delete(path)` `:210` / `delete_with_policy` `:224`
  - `rename(from,to)` `:259` / `rename_with_policy` `:273`
  - `copy(from,to)` `:309` / `copy_with_policy` `:322`
  - `download_stream(path) -> BytesStream` `:398` / `download_stream_with_policy` `:409`
  - `upload_stream(path, BytesStream)` `:438` / `upload_stream_with_policy` `:449`
  - `as_store(name) -> Option<&dyn StoreDriver>` `:348`; `as_store_err(name) -> StorageResult<&dyn StoreDriver>` `:370`
- `enum StorageError { StoreNotFound, Store(opendal), UnableToReadBytes, Multi(BTreeMap), Any }` — `:28`
- `type StorageResult<T>` — `:45`
- `trait StoreDriver` — `src/storage/drivers/mod.rs:62`: `upload`, `get`, `delete`, `rename`, `copy`, `exists`, `get_stream` (default), `upload_stream` (default). `UploadResponse { e_tag, version }` `:21`; `GetResponse::{bytes, into_stream}` `:34`.
- `BytesStream` — `src/storage/stream.rs:10`: `collect()` `:39`, `into_body()` → axum Body `:63`, `from_body_stream()` `:72`, impl `Stream`.
- `Contents` — `src/storage/contents.rs:4`: `From<Bytes>`, `Into<Vec<u8>>`, `TryFrom -> String`.

### 2.3 DRIVERS (enumerated — ALL of them)
All wrap `OpendalAdapter` (`src/storage/drivers/opendal_adapter.rs:11`), which adds an OpenDAL `RetryLayer::default().with_jitter()` (`:21`) and provides native streaming + rename/copy capability fallbacks (`:82,103`).

| Driver | Module | Constructor(s) `file:line` | Feature flag | Notes |
|--------|--------|----------------------------|--------------|-------|
| **Local / filesystem** | `drivers/local.rs` | `new()` (root `/`) `:18`; `new_with_prefix(prefix)` `:38` | none (always on) | OpenDAL `Fs` |
| **In-memory** | `drivers/mem.rs` | `new()` `:18` | none (always on) | OpenDAL `Memory` |
| **Null** (default) | `drivers/null.rs` | `new()` `:19` | none | Every op returns `StorageError::Any("Operation not supported by null storage")` |
| **AWS S3** | `drivers/aws.rs` | `new(bucket, region)` `:28`; `with_credentials(bucket,region,cred)` `:89`; `with_credentials_and_endpoint(bucket,region,endpoint,cred)` `:54`; `with_failure()` (test-only) `:112` | `storage_aws_s3` | `struct Credential { key_id, secret_key, token: Option }` `:8` |
| **Azure Blob** | `drivers/azure.rs` | `new(container, account_name, access_key, endpoint)` `:17` | `storage_azure` | OpenDAL `Azblob` |
| **GCP GCS** | `drivers/gcp.rs` | `new(bucket, credential_path)` `:17` | `storage_gcp` | OpenDAL `Gcs` |

Driver modules are cfg-gated in `drivers/mod.rs:7-16` (aws/azure/gcp behind features; local/mem/null/opendal_adapter always compiled).

### 2.4 STRATEGIES (enumerated — ALL of them) — `src/storage/strategies/`
- `trait StorageStrategy` — `strategies/mod.rs:12`: upload/download/delete/rename/copy + download_stream/upload_stream.
1. **SingleStrategy** — `strategies/single.rs:13`. `new(primary: &str)` `:21`. Field `primary: String`. All ops go to one store.
2. **MirrorStrategy** — `strategies/mirror.rs:40`. `new(primary, secondaries: Option<Vec<String>>, failure_mode)` `:297`. `FailureMode { MirrorAll, AllowMirrorFailure }` `:31`. Download: try primary then fall through secondaries (`:94`). `should_fail` `:318`. Stream: download from primary only (`:220`); upload buffers then fans out.
3. **BackupStrategy** — `strategies/backup.rs:47`. `new(primary, secondaries, failure_mode)` `:264`. `FailureMode { BackupAll, AllowBackupFailure, AtLeastOneFailure, CountFailure(usize) }` `:32`. Download: primary ONLY (`:91`). `download_stream`: primary only (`:208`); `upload_stream`: buffers via `collect()` then uploads to primary+secondaries (`:217`).
- Secondary failures aggregate into `StorageError::Multi(BTreeMap<store,err>)`.

### 2.5 Feature flags (`Cargo.toml:51-54`)
- `all_storage = ["storage_aws_s3","storage_azure","storage_gcp"]`
- `storage_aws_s3 = ["opendal/services-s3"]`
- `storage_azure = ["opendal/services-azblob"]`
- `storage_gcp = ["opendal/services-gcs"]`
- None are in `default`. Local/mem/null work with no feature.

### 2.6 Config
- **No YAML config for storage.** Storage is configured purely in Rust via the `after_context` hook returning `AppContext { storage: ... }`. Cloud credentials are passed as function args (not env-var-driven by the framework, though the S3 `Credential` fields correspond to `AWS_ACCESS_KEY_ID` etc. per doc-comment `aws.rs:9-14`).

### 2.7 Generator / CLI
- No storage generator. (docs show a manual controller upload example only.)

### 2.8 Doc coverage — `docs-site/content/docs/infrastructure/storage.md` → **THIN / partially STALE**
1. **Streaming API entirely undocumented.** `download_stream`/`upload_stream`/`BytesStream`/`into_body()` are a full public feature (memory-efficient large files, axum Body integration) with zero doc coverage. Significant gap.
2. **Driver constructor coverage is incomplete/inaccurate.** Doc's multi-driver example calls `drivers::aws::new("users")` with ONE arg — real signature is `new(bucket, region)` (2 args). Azure `new` needs 4 args, GCP 2 args, `local::new_with_prefix` unmentioned, S3 `with_credentials*` unmentioned.
3. **Strategy constructor drift.** Doc uses `MirrorStrategy::new("store_1", Some(vec![...]), FailureMode::MirrorAll)` — signature matches (`:297`), OK; but `BackupStrategy` `AtLeastOneFailure`/`CountFailure` modes are listed (accurate).
4. `after_context` "Setup" example is a no-op stub and the `Storage::single` example is correct.
5. Glossary uses `StorageDriver` — actual trait is `StoreDriver` (naming drift).
6. Testing example uses `request::<App>(...)` — signature likely stale vs mailer doc's `request::<App, Migrator, _, _>`; worth cross-checking against testing module.
7. `UploadResponse { e_tag, version }` and per-store direct access (`as_store`) not documented.

---

## 3. CACHE

### 3.1 Purpose
Generic key→string cache with JSON (serde) value serialization, TTL support, and get-or-insert. Default `Null` driver. Exposed as `AppContext.cache`.

### 3.2 Public API — `src/cache/mod.rs`
- `struct Cache { driver: Box<dyn CacheDriver> }` — `:65`; `Cache::new(driver)` `:73`
- `create_cache_provider(&config::Config) -> Result<Arc<Cache>>` — `:45` (matches `CacheConfig` variants → redis/inmem/null)
- Methods (all `async`):
  - `ping()` `:93`
  - `contains_key(key)` `:113`
  - `get::<T: DeserializeOwned>(key) -> Option<T>` `:153`
  - `insert::<T: Serialize>(key, &T)` `:201`
  - `insert_with_expiry::<T>(key, &T, Duration)` `:250`
  - `get_or_insert::<T,F>(key, future)` `:305`
  - `get_or_insert_with_expiry::<T,F>(key, Duration, future)` `:364`
  - `remove(key)` `:401`
  - `clear()` `:422`
- `enum CacheError { Any, Serialization, Deserialization, Redis(cfg redis), RedisConnectionError(cfg redis) }` — `:18`
- `type CacheResult<T>` — `:37`
- `trait CacheDriver` — `src/cache/drivers/mod.rs:18`: ping, contains_key, get, insert, insert_with_expiry, remove, clear (all string-valued).

### 3.3 DRIVERS (enumerated — ALL of them) — `src/cache/drivers/`
| Driver | Module | Constructor `file:line` | Feature flag | Notes |
|--------|--------|-------------------------|--------------|-------|
| **Null** (default) | `drivers/null.rs` | `new() -> Box<dyn CacheDriver>` `:24` | none (always on) | `get()` returns `Ok(None)`; ALL other ops (ping/contains_key/insert/insert_with_expiry/remove/clear) return `Err("Operation not supported by null cache")` |
| **In-memory** | `drivers/inmem.rs` | `new(&InMemCacheConfig) -> Cache` `:23`; `Inmem::from(moka::Cache)` `:44` | `cache_inmem` (in `default`) | Backed by `moka::sync::Cache`; per-entry expiry via `Expiration { Never, AfterDuration }` + `InMemExpiry` (`:139,157`) |
| **Redis** | `drivers/redis.rs` | `async new(&RedisCacheConfig) -> CacheResult<Cache>` `:27`; `Redis::from(Pool)` `:50` | `cache_redis` | `bb8` + `bb8_redis` pool; `clear()` issues `FLUSHDB` (`:140`) |

Driver modules cfg-gated in `drivers/mod.rs:10-14` (inmem behind `cache_inmem`, redis behind `cache_redis`, null always).

### 3.4 Config (`src/config/cache.rs`)
- `enum CacheConfig` (serde `tag = "kind"`, `#[default] Null`) — `:6`
  - `InMem(InMemCacheConfig)` (cfg `cache_inmem`)
  - `Redis(RedisCacheConfig)` (cfg `cache_redis`)
  - `Null` (default)
- `InMemCacheConfig { max_capacity: u64 }` — `:19`; default `32 * 1024 * 1024` = 33554432 (`:24`)
- `RedisCacheConfig { uri: String, max_size: u32 }` — `:29`
- YAML: `cache.kind: {Null|InMem|Redis}`, `cache.max_capacity`, `cache.uri`, `cache.max_size`. No storage-specific env vars (values may use Tera `get_env` in YAML).

### 3.5 Feature flags (`Cargo.toml:56-57`)
- `cache_inmem = ["dep:moka"]` — **enabled in `default`** (`Cargo.toml:32`).
- `cache_redis = ["dep:bb8-redis","dep:bb8"]` — not default.

### 3.6 Generator / CLI
- None for cache.

### 3.7 Doc coverage — `docs-site/content/docs/infrastructure/cache.md` (116 lines) → **THIN but mostly ACCURATE**
It DOES undersell the API. Missing/thin:
1. **`get_or_insert_with_expiry`** not shown (only `get_or_insert`).
2. **`ping()`** and **`clear()`** methods not documented.
3. **`insert` on `?Sized` / string** nuance fine, but the trait-level `CacheDriver` (custom drivers) not mentioned.
4. Redis `clear()` = `FLUSHDB` (flushes the whole DB, not just app keys) — an important operational caveat, undocumented.
5. Null driver description is ACCURATE (get→None, others error) and matches `null.rs`.
6. `max_capacity` default value (33554432 / 32MiB) documented correctly.
7. `CacheError` variants / error handling not documented.
Overall: accurate as far as it goes, but ~5 real API methods + the FLUSHDB caveat are missing.

---

## 4. 1.0-RELEVANT NOTES
- **Mailer TLS overhaul (0.17)**: `MailerTls::{Starttls,Implicit,None}` + `tls_mode()` precedence is a headline change. Docs must add implicit-TLS/465 guidance before 1.0 — current docs actively mislead (imply `secure:true` covers TLS providers).
- **Email threading headers** (`EmailHeaders`) and cc/bcc are stable public API, undocumented — good 1.0 doc additions.
- **Storage streaming** (`BytesStream`, `download_stream`/`upload_stream`, axum `into_body()`) is a substantial undocumented capability; prioritize for 1.0.
- **OpenDAL coupling**: storage is entirely OpenDAL-backed (opendal 0.57 per comments); `StorageError::Store` boxes `opendal::Error`. Public surface deliberately hides OpenDAL (`BytesStream` wraps `Reader`). Version pinning noted in aws.rs comments (`skip_signature` rename).
- **Naming inconsistencies** to reconcile for 1.0 docs: trait is `StoreDriver` (docs say `StorageDriver`); `as_store_err` flagged in-code as awkward (`mod.rs:369 REVIEW`).
- **No storage YAML config** — cloud storage is code-only (`after_context`); this is a notable UX gap vs cache/mailer which are YAML-driven.
- Redis cache errors are feature-gated into `CacheError`; building without `cache_redis` changes the enum shape.
