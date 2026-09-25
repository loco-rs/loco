---
title: З'єднання з Postgres та Redis через TLS
description: Підключайтеся до керованих Postgres (RDS, Supabase, Neon) та керованих Redis (ElastiCache, Upstash) через зашифровані TLS-з'єднання.
sidebar:
  order: 23
---

Мета: з'єднати ваш застосунок із **керованою** базою даних або кешем, що вимагає (чи має використовувати) шифрування в дорозі. Більшість хмарних провайдерів — AWS RDS/ElastiCache, Supabase, Neon, Azure, Upstash — або вимагають TLS, або наполегливо його рекомендують.

Loco використовує [rustls](https://github.com/rustls/rustls) з чистим Rust-провайдером `ring` для всіх TLS-шляхів, тож жодне з цього не потребує системного OpenSSL чи C-тулчейна.

## Postgres через TLS

TLS для Postgres працює з коробки, щоразу коли увімкнена фіча `with-db` (стандарт для застосунків із базою даних) — **не потрібно жодної Cargo-фічі та жодного коду**. Ви вмикаєте його повністю через URL з'єднання в `config/*.yaml`, використовуючи ті самі параметри `sslmode` / `sslrootcert`, які розуміють `libpq` і кожен клієнт Postgres.

```yaml
# config/production.yaml
database:
  # Вимагати зашифроване з'єднання; збій, якщо сервер не підтримує TLS.
  uri: "postgres://user:pass@db.example.com:5432/myapp?sslmode=require"
```

`sslmode` приймає стандартні значення, від найслабшого до найсильнішого:

| `sslmode` | Зашифровано? | Перевіряє сервер? | Використовуйте, коли |
|---|---|---|---|
| `disable` | ні | ні | лише локально/dev |
| `prefer` | якщо доступно | ні | — |
| `require` | так | ні | шифрування без перевірки сертифікатів |
| `verify-ca` | так | ланцюжок CA | ви довіряєте CA |
| `verify-full` | так | ланцюжок CA **і** ім'я хоста | рекомендовано для production |

Для `verify-ca` / `verify-full` із провайдером, чиє CA відсутнє у вбудованому сховищі кореневих сертифікатів, вкажіть CA-бандл, який вони вам дають, і додайте шляхи клієнтських сертифікатів для mutual TLS:

```yaml
database:
  uri: "postgres://user:pass@db.example.com:5432/myapp?sslmode=verify-full&sslrootcert=/etc/ssl/rds-ca.pem"
  # Для mTLS, також: &sslcert=/path/client.crt&sslkey=/path/client.key
```

Швидкий довідник провайдерів (усі підтримують `sslmode=require`; для найсильнішого налаштування використовуйте `verify-full` + їхнє CA):

- **AWS RDS/Aurora** — завантажте [CA-бандл RDS](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/UsingWithRDS.SSL.html) і передайте його як `sslrootcert`.
- **Supabase / Neon** — TLS обов'язковий; `sslmode=require` працює напряму, `verify-full` з їхнім опублікованим CA — сильніше.
- **Azure Database for PostgreSQL** — вимагає TLS; використовуйте `sslmode=require` або суворіше.

> **Усунення несправностей:** помилка `server does not support TLS` означає, що **сервер** відмовив у TLS-узгодженні (неправильний хост/порт або TLS вимкнено на боці сервера) — це не баг Loco чи клієнта. Перевірте, що ви вказуєте на TLS-ендпоінт провайдера.

### Черга Postgres через TLS

Якщо ви використовуєте бекенд черги на **Postgres** (фіча `worker`), направлений на керований Postgres, що допускає лише TLS, пул воркерів має власний TLS-бекенд rustls, тож той самий URL із `sslmode=...` у `queue.uri` працює і там — включно зі збіркою лише для воркерів, яка не вмикає `with-db`.

## Redis через TLS

TLS для Redis — опційний за Cargo-фіччю, оскільки базовий клієнт Redis за замовчуванням не компілює TLS-стек.

1. Увімкніть фічу `redis_tls` разом із вашою Redis-фіччю:

```toml
# Cargo.toml
loco-rs = { version = "*", features = ["worker_redis", "redis_tls"] }
# або, для бекенду кешу Redis:
# loco-rs = { version = "*", features = ["cache_redis", "redis_tls"] }
```

`redis_tls` озброює **одночасно** чергу воркерів і Redis-шлях кешу (вони спільно використовують той самий нижчий клієнт), використовуючи вбудовані корені webpki, тож це працює у slim/distroless контейнерних образах без системного сховища сертифікатів.

2. Використовуйте схему `rediss://` (зверніть увагу на подвійне `s`) у вашій конфігурації — це єдина зміна на боці конфігурації:

```yaml
# config/production.yaml
queue:
  kind: Redis
  uri: "rediss://:password@my-redis.example.com:6380"

# і/або кеш:
cache:
  kind: Redis
  uri: "rediss://:password@my-redis.example.com:6380"
```

Примітки щодо провайдерів:

- **AWS ElastiCache** — увімкніть «encryption in-transit» на кластері, потім використовуйте `rediss://` з auth-токеном як паролем.
- **Upstash / Redis Cloud** — TLS-ендпоінти за замовчуванням використовують `rediss://`; скопіюйте URL з дашборда.
