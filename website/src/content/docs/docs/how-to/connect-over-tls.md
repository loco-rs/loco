---
title: Connect to Postgres and Redis over TLS
description: Reach managed Postgres (RDS, Supabase, Neon) and managed Redis (ElastiCache, Upstash) over encrypted TLS connections.
sidebar:
  order: 23
---

Goal: connect your app to a **managed** database or cache that requires (or should use) encryption in transit. Most cloud providers — AWS RDS/ElastiCache, Supabase, Neon, Azure, Upstash — either require TLS or strongly recommend it.

Loco uses [rustls](https://github.com/rustls/rustls) with the pure-Rust `ring` provider for every TLS path, so none of this needs a system OpenSSL or a C toolchain.

## Postgres over TLS

Postgres TLS works out of the box whenever the `with-db` feature is on (the default for database apps) — there is **no Cargo feature to enable and no code to write**. You turn it on entirely through the connection URL in `config/*.yaml`, using the same `sslmode` / `sslrootcert` parameters `libpq` and every Postgres client understand.

```yaml
# config/production.yaml
database:
  # Require an encrypted connection; fail if the server won't do TLS.
  uri: "postgres://user:pass@db.example.com:5432/myapp?sslmode=require"
```

`sslmode` accepts the standard values, from weakest to strongest:

| `sslmode` | Encrypted? | Verifies the server? | Use when |
|---|---|---|---|
| `disable` | no | no | local/dev only |
| `prefer` | if available | no | — |
| `require` | yes | no | encryption without certificate checks |
| `verify-ca` | yes | CA chain | you trust the CA |
| `verify-full` | yes | CA chain **and** hostname | recommended for production |

For `verify-ca` / `verify-full` against a provider whose CA is not in the bundled root store, point at the CA bundle they give you, and add client-certificate paths for mutual TLS:

```yaml
database:
  uri: "postgres://user:pass@db.example.com:5432/myapp?sslmode=verify-full&sslrootcert=/etc/ssl/rds-ca.pem"
  # For mTLS, also: &sslcert=/path/client.crt&sslkey=/path/client.key
```

Provider quick reference (all support `sslmode=require`; use `verify-full` + their CA for the strongest setting):

- **AWS RDS/Aurora** — download the [RDS CA bundle](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/UsingWithRDS.SSL.html) and pass it as `sslrootcert`.
- **Supabase / Neon** — TLS is required; `sslmode=require` works directly, `verify-full` with their published CA is stronger.
- **Azure Database for PostgreSQL** — requires TLS; use `sslmode=require` or stricter.

> **Troubleshooting:** the error `server does not support TLS` means the **server** refused the TLS negotiation (wrong host/port, or TLS disabled server-side) — it is not a Loco or client bug. Check that you are pointing at the provider's TLS endpoint.

### Postgres queue over TLS

If you use the Postgres **queue** backend (`worker` feature) pointed at a TLS-only managed Postgres, the worker pool carries its own rustls TLS backend, so the same `sslmode=...` URL in `queue.uri` works there too — including in a worker-only build that does not enable `with-db`.

## Redis over TLS

Redis TLS is opt-in behind a Cargo feature, because the base Redis client does not compile a TLS stack by default.

1. Enable the `redis_tls` feature alongside your Redis feature:

```toml
# Cargo.toml
loco-rs = { version = "*", features = ["worker_redis", "redis_tls"] }
# or, for the Redis cache backend:
# loco-rs = { version = "*", features = ["cache_redis", "redis_tls"] }
```

`redis_tls` arms **both** the worker queue and the cache Redis paths at once (they share the same underlying client), using webpki-bundled roots so it works in slim/distroless container images with no system certificate store.

2. Use the `rediss://` scheme (note the double `s`) in your config — that is the only change on the config side:

```yaml
# config/production.yaml
queue:
  kind: Redis
  uri: "rediss://:password@my-redis.example.com:6380"

# and/or the cache:
cache:
  kind: Redis
  uri: "rediss://:password@my-redis.example.com:6380"
```

Provider notes:

- **AWS ElastiCache** — enable "encryption in-transit" on the cluster, then use `rediss://` with the auth token as the password.
- **Upstash / Redis Cloud** — TLS endpoints are `rediss://` by default; copy the URL from the dashboard.
