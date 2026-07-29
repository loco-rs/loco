+++
title = "Deploy to production"
description = "Build a release binary, generate a Dockerfile or nginx config with cargo loco generate deployment, and review production config before shipping."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 32
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/infrastructure/deployment/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: get a Loco app running on a production host. Loco compiles to a single self-contained binary — the target server needs neither `cargo` nor a Rust toolchain, just the binary and a `config/` folder.

## 1. Build the release binary

```sh
cargo build --release
```

Your binary name matches the `[package] name` in `Cargo.toml` (with a `-cli` suffix, e.g. `myapp-cli`), and lands in `./target/release/`.

## 2. Generate a Dockerfile (optional)

```sh
cargo loco generate deployment docker
```

`kind` is a **positional** argument — `docker` or `nginx`, not a `--kind` flag.

This writes two files to your project root:

- `Dockerfile` — multi-stage build: compiles with `cargo build --release` in a `rust:slim` builder stage, then copies just the compiled binary and `config/` into a slim `debian:bookworm-slim` runtime image. If your app has a `frontend/package.json` (client-side rendering), it also installs Node and runs `npm install && npm run build` in the builder stage. If `server.middlewares.static_assets` is configured, the folders it points to are copied into the final image too.
- `.dockerignore` — excludes `target/`, `.git`, and other build artifacts from the Docker build context.

Build and run it like any other image:

```sh
docker build -t myapp .
docker run -p 5150:5150 --env-file .env myapp
```

## 3. Generate an nginx config (optional)

```sh
cargo loco generate deployment nginx
```

This writes `nginx/default.conf`, a reverse-proxy config derived from your current `server.host` / `server.port` (`config/<env>.yaml`) — it proxies both the bare domain and wildcard subdomains to your app.

## 4. Review production config

There's no separate "production mode" — Loco picks a config file by environment (`config/production.yaml` by default, or override with `LOCO_ENV`). Before deploying, walk through these sections:

**Logger** — turn `pretty_backtrace` off (it's development-friendly, not performance-friendly) and prefer `json` for log aggregation:

```yaml
logger:
  enable: true
  pretty_backtrace: false
  level: info
  format: json
```

See [Configure logging](@/docs/how-to/configure-logging.md) for the full picture.

**Server** — bind to all interfaces and inject the port from the environment:

```yaml
server:
  port: {{ get_env(name="NODE_PORT", default=5150) }}
  host: {{ get_env(name="APP_HOST", default="http://localhost") }}
```

**Database** — real connection limits, no destructive flags:

```yaml
database:
  uri: "{{ get_env(name='DATABASE_URL', default='postgres://loco:loco@localhost:5432/loco_app') }}"
  enable_logging: false
  connect_timeout: 500
  idle_timeout: 500
  min_connections: 1
  max_connections: 10
  auto_migrate: true
  dangerously_truncate: false
  dangerously_recreate: false
```

**Auth secret** — inject via environment, never hardcode:

```yaml
auth:
  jwt:
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
```

**Queue / mailer** — same pattern: point `uri`/`host` at env vars. See the [Configuration reference](@/docs/reference/configuration.md) for every key across all of these sections.

## 5. Run `loco doctor` before going live

```sh
myapp-cli doctor --production
```

`doctor` validates DB/cache/queue connectivity against the config it would actually load. Add `-c`/`--config` to also print the fully-resolved config for inspection:

```sh
myapp-cli doctor --config --production
```

## 6. Ship it

Copy the binary and the `config/` folder to the server (no source, no `Cargo.lock`, no toolchain needed):

```sh
scp target/release/myapp-cli config/ user@server:/opt/myapp/
ssh user@server '/opt/myapp/myapp-cli start'
```

## Verify

- `myapp-cli doctor --production` exits 0 and reports all checks passing.
- `myapp-cli start` boots and the startup banner shows the environment, DB, and logger you expect.
- Hitting the app's health/root route through nginx (if you generated one) returns a response, confirming the reverse proxy is wired to the right host/port.

## Reference

- `generate deployment` CLI shape (`docker`/`nginx` as `kind`): [CLI reference](@/docs/reference/cli.md)
- Every config key referenced above (`logger`, `server`, `database`, `auth`, `mailer`, `queue`): [Configuration reference](@/docs/reference/configuration.md)
