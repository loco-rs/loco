---
title: Run SQLite in production
description: What Loco's SQLite defaults do, the single-writer caveat, backing up with Litestream, and running without Redis using the SQL queue and the database cache.
sidebar:
  order: 33
---

Goal: run an app on one server with one SQLite file as its database, job queue and cache, and keep that file backed up. Rails 8 takes the same route.

## 1. Point the app at a file

```yaml
# config/production.yaml
database:
  uri: sqlite:///var/lib/myapp/app.sqlite?mode=rwc
  enable_logging: false
  min_connections: 1
  max_connections: 8
  connect_timeout: 500
  idle_timeout: 500

queue:
  kind: Sqlite
  uri: sqlite:///var/lib/myapp/queue.sqlite?mode=rwc

cache:
  kind: Database
```

Put the file on a persistent volume, not in the container's writable layer. The job queue gets its own file so job polling never competes with your requests for the app database's write lock. You can also point it at the app file.

## 2. What Loco sets on every connection

Loco applies these PRAGMAs to every connection of the app database pool and of the SQLite queue pool, not only the first one:

| PRAGMA | Value | Why |
|---|---|---|
| `journal_mode` | `WAL` | Readers do not block the writer, and the writer does not block readers. Skipped for `:memory:` databases. |
| `synchronous` | `NORMAL` | Safe with WAL: a power cut can lose the last commits but not corrupt the file. |
| `busy_timeout` | `5000` | A writer that finds the lock taken waits up to 5 seconds instead of failing with `database is locked`. |
| `foreign_keys` | `ON` | SQLite ignores foreign keys unless asked, per connection. |
| `journal_size_limit` | `67108864` | Keeps the WAL file from staying huge after a big write. |
| `mmap_size` | `134217728` | Memory-mapped reads. |
| `cache_size` | `2000` | Pages of page cache per connection. |

Override or add PRAGMAs in config. Your values win over the defaults:

```yaml
database:
  sqlite:
    pragmas:
      synchronous: FULL
      cache_size: -20000 # a negative number is KiB
      temp_store: MEMORY

queue:
  kind: Sqlite
  uri: sqlite:///var/lib/myapp/queue.sqlite?mode=rwc
  pragmas:
    synchronous: FULL
```

`database.run_on_start` still works. It runs a SQL string once, after the pool is up, and so only on one connection. Use `sqlite.pragmas` for settings.

## 3. The single-writer caveat

SQLite lets many connections read at once but only one write at a time, per file. With WAL and `busy_timeout`, writers queue up behind each other instead of failing, and that is fine for the traffic a single server handles. What goes wrong:

- **Long transactions.** A transaction that holds the write lock while it calls an HTTP API stalls every other writer. Keep write transactions short, and do slow work before you begin them.
- **Big pools.** More connections do not mean more writes. `max_connections` of 5 to 10 is plenty.
- **More than one machine.** The file lives on one disk. Do not put it on a network filesystem and do not run two app servers against it. If you need that, use Postgres.
- **Multiple processes on one machine are fine**, for example a web process and a worker process that share the same file.

## 4. Back up with Litestream

[Litestream](https://litestream.io) streams the WAL to object storage (S3, GCS, Azure, SFTP, a local path) continuously, so you can restore to within a second or so of the last write. Copying the `.sqlite` file by hand while the app runs is not a backup: you miss the WAL.

```yaml
# litestream.yml
dbs:
  - path: /var/lib/myapp/app.sqlite
    replicas:
      - url: s3://my-bucket/myapp/app
  - path: /var/lib/myapp/queue.sqlite
    replicas:
      - url: s3://my-bucket/myapp/queue
```

Credentials come from `LITESTREAM_ACCESS_KEY_ID` and `LITESTREAM_SECRET_ACCESS_KEY` (or the usual AWS variables). Skip the queue entry if you do not mind losing queued jobs, or if your jobs are recreated on boot.

### As a sidecar

The Dockerfile from `cargo loco generate deployment docker` builds the app image only. Run Litestream next to it with the same volume:

```yaml
# docker-compose.yml
services:
  app:
    build: .
    command: ["start"]
    environment:
      DATABASE_URL: sqlite:///data/app.sqlite?mode=rwc
    volumes:
      - data:/data
  litestream:
    image: litestream/litestream
    command: ["replicate"]
    volumes:
      - data:/data
      - ./litestream.yml:/etc/litestream.yml
    environment:
      LITESTREAM_ACCESS_KEY_ID: ${LITESTREAM_ACCESS_KEY_ID}
      LITESTREAM_SECRET_ACCESS_KEY: ${LITESTREAM_SECRET_ACCESS_KEY}
volumes:
  data:
```

Both containers see the same files, and WAL mode is what makes it safe for Litestream to read them while the app writes.

### Inside the app image

To ship one container instead, add Litestream to the runtime stage of the generated Dockerfile and let it start the app with `-exec`, so that the container stops when the app does:

```dockerfile
FROM debian:bookworm-slim
WORKDIR /usr/app
COPY --from=litestream/litestream:latest /usr/local/bin/litestream /usr/local/bin/litestream
COPY litestream.yml /etc/litestream.yml
# ... the COPY lines the generator wrote ...
ENTRYPOINT ["litestream", "replicate", "-exec", "/usr/app/myapp-cli start"]
```

### Restore

On a new machine, before the app starts:

```sh
litestream restore -if-db-not-exists -if-replica-exists -config /etc/litestream.yml /var/lib/myapp/app.sqlite
```

Run it as the container's first step, and a fresh volume comes back with the data it had.

## 5. No Redis needed

| You used to need Redis for | Now |
|---|---|
| Background jobs | `queue.kind: Sqlite` (or `Postgres`). See [Choose a queue backend](/docs/how-to/choose-queue-backend). |
| A shared cache | `cache.kind: Database`. See [Use the cache](/docs/how-to/use-cache). |

Both keep their state in the database, so the Litestream backup above covers jobs and cached entries too.

## Reference

- `database.sqlite.pragmas`, `queue.pragmas`, `cache.kind: Database`: [Configuration reference](/docs/reference/configuration)
- Deploying the binary: [Deploy](/docs/how-to/deploy)
