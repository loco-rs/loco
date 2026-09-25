---
title: Feature-прапорці
description: "Повна матриця Cargo feature у loco-rs: значення за замовчуванням, що вмикає кожен прапорець і як прапорці взаємодіють між собою."
sidebar:
  order: 4
---

`loco-rs` керує більшістю опціональної функціональності через Cargo feature, оголошені в кореневому `Cargo.toml:27-64`. Ця сторінка — вичерпна матриця: кожен прапорець, його стан за замовчуванням, що він вмикає та як прапорці взаємодіють між собою і з `cargo loco`.

## Значення за замовчуванням

```toml
default = ["auth", "cli", "with-db", "cache_inmem", "worker", "db-sqlite"]
```

Звичайна залежність `loco-rs = "..."` (без `default-features = false`) підтягує JWT-автентифікацію, CLI `cargo loco`, підтримку бази даних Sea-ORM, кеш у пам'яті та workers черги на базі Postgres/SQLite. Worker черги на базі Redis (`worker_redis`) **не** входить у набір за замовчуванням — підключайте його явно, якщо ваш застосунок використовує чергу Redis.

## Матриця

| Прапорець | За замовчуванням | Вмикає (залежності / під-feature) | Призначення |
|---|---|---|---|
| `auth` | **УВІМК** | `dep:jsonwebtoken`, `jsonwebtoken/rust_crypto` | JWT-автентифікація. Обирає чисто Rust-бекенд `rust_crypto` у `jsonwebtoken` (jsonwebtoken 10 більше не постачається з crypto-бекендом за замовчуванням), тому прапорець залишається самодостатнім і не потребує C-тулчейна, навіть коли ввімкнений сам із `default-features = false`. |
| `cli` | **УВІМК** | `dep:clap` | Вмикає runtime CLI `cargo loco` (`src/cli.rs`). |
| `with-db` | **УВІМК** | `dep:sea-orm`, `dep:sea-orm-migration`, `dep:sqlx`, `loco-gen/with-db` | Підтримка бази даних Sea-ORM 2.0. Керує CLI-субкомандою `db` та генераторами, що залежать від БД (`model`, `migration`, `scaffold`). Лише драйвер Postgres, якщо також не ввімкнено `db-sqlite`. |
| `db-sqlite` | **УВІМК** | `sea-orm?/sqlx-sqlite`, `sea-orm-migration?/sqlx-sqlite`, `sqlx?/sqlite` | Драйвери SQLite для SeaORM/sqlx та бекенд черги на SQLite. Вимкніть через `default-features = false` у застосунках лише з Postgres, щоб пропустити витрати на компіляцію `libsqlite3-sys`. |
| `multi-tenancy` | вимк | `with-db` | Явний tenant-скоупінг через `TenantEntity`, `TenantQueryExt` та `TenantActiveModelExt`. Див. [Додати row-level мультиорендність](/uk/docs/how-to/multi-tenancy/). |
| `testing` | вимк | `dep:axum-test`, `dep:scraper`, `dep:tree-fs` | Утиліти тестового оточення. Вмикається разом із `multi-tenancy` для docs.rs та використовується власними `dev-dependencies` crate'а. |
| `cache_inmem` | **УВІМК** | `dep:moka` | Кеш-бекенд у пам'яті. |
| `cache_redis` | вимк | `dep:bb8-redis`, `dep:bb8` | Пул кешу на базі Redis. |
| `worker` | **УВІМК** | `dep:sqlx`, `dep:ulid` | Черга/workers фонових завдань, бекенд Postgres (SQLite, коли також увімкнено `db-sqlite`). Який саме запускається, обирається в runtime через `queue.kind` у конфігурації (`Postgres` або `Sqlite`), а не окремим прапорцем для кожної БД. |
| `worker_redis` | вимк | `worker`, `dep:redis` | Додає бекенд черги на базі Redis поверх `worker` (має на увазі його). Увімкніть, якщо `queue.kind` вашого застосунку — `Redis`. |
| `redis_tls` | вимк | `redis/tokio-rustls-comp`, `redis/tls-rustls-webpki-roots`, `dep:rustls` | Redis через TLS (`rediss://` URL) для керованих провайдерів, як-от ElastiCache, Upstash або Azure Cache. Озброює одночасно і worker-, і cache-шлях Redis — вони спільно використовують той самий crate `redis` — коренями з webpki-бандла, тому залишається переносимим у slim/distroless-образах. Увімкніть разом із `worker_redis`/`cache_redis` і вкажіть у конфігурації `rediss://` URL; зміни коду не потрібні. |
| `all_storage` | вимк | `storage_aws_s3` + `storage_azure` + `storage_gcp` | Парасольковий прапорець — вмикає всі хмарні бекенди сховища одразу. |
| `storage_aws_s3` | вимк | `opendal/services-s3` | Бекенд сховища AWS S3. |
| `storage_azure` | вимк | `opendal/services-azblob` | Бекенд сховища Azure Blob. |
| `storage_gcp` | вимк | `opendal/services-gcs` | Бекенд сховища Google Cloud Storage. |
| `embedded_assets` | вимк | (порожньо — build-time прапорець) | Вбудовує директорію `assets/` застосунку у скомпільований бінарник і відповідно змінює шлях завантаження ассетів у view-рушії, замість читання ассетів з диска в runtime. |

Джерело: кореневий `Cargo.toml:27-64`.

## Взаємодії

- **`worker` розблоковує субкоманду `jobs`.** `cargo loco jobs` (разом із її субкомандами `cancel`/`tidy`/`purge`/`dump`/`import`/`requeue`) компілюється щоразу, коли ввімкнено feature `worker` (`#[cfg(feature = "worker")]`, `src/cli.rs:27`). Той самий cfg керує імпортом `JobStatus`, який використовується механізмом jobs. Оскільки `worker_redis` має на увазі `worker`, CLI `jobs` доступний для будь-якого бекенду черги — Redis, Postgres або SQLite.
- **`debug_assertions` (не Cargo feature) керує `generate` та `db entities`.** Субкоманди `cargo loco generate` та `db entities` компілюються лише в debug-збірках (`#[cfg(debug_assertions)]`, `src/cli.rs:29, 140, 173`). Вони недоступні у збірках `--release` незалежно від увімкнених Cargo feature.
- **`all_storage` — суто парасольковий прапорець.** Він не має власних залежностей; просто вмикає `storage_aws_s3`, `storage_azure` та `storage_gcp` разом.
- **`multi-tenancy` має на увазі `with-db`.** Помічники tenant-ів будуються на Sea-ORM; саме по собі ввімкнення `with-db` їх не вмикає.
- **`auth` обирає `jsonwebtoken/rust_crypto`.** Оскільки jsonwebtoken 10 відокремив свій crypto-бекенд, `auth` явно вмикає під-feature `rust_crypto`, щоб підтримка JWT працювала без системного C-тулчейна (наприклад, OpenSSL).
- **`with-db` — це передумова, а не наслідок.** Увімкнення `worker` само по собі не підтягує `with-db`; це два незалежні прапорці, які просто спільно використовують залежність `sqlx`.
- **Бекенд черги обирається в runtime, а не feature-прапорцем.** `worker` компілює провайдери черги Postgres і SQLite; який саме запускається, вирішує `queue.kind` (`Postgres` або `Sqlite`) у конфігурації вашого застосунку. `worker_redis` додає провайдер Redis, який обирається так само через `queue.kind: Redis`. Див. [Обрати бекенд черги](/uk/docs/how-to/choose-queue-backend/).

## Вимкнення значень за замовчуванням

Щоб відмовитися від набору за замовчуванням (наприклад, для застосунку без БД), підключайте залежність з `default-features = false` і повторно перелічіть лише потрібні прапорці:

```toml
loco-rs = { version = "...", default-features = false, features = ["cli"] }
```

Саме цей патерн використовує сам генератор `loco new`, коли застосунок створюється без бази даних (див. потік створення застосунку в CLI-довіднику): він генерує `default-features = false` з `features = ["cli"]`, плюс `worker_redis`, якщо було обрано чергу на базі Redis, `worker`, якщо було обрано чергу на базі Postgres, або `worker` і `db-sqlite`, якщо було обрано чергу на базі SQLite.
