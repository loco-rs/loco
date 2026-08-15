---
title: Deploy to production
description: Build a release binary, generate a Dockerfile or nginx config with cargo loco generate deployment, and review production config before shipping.
sidebar:
  order: 32
---

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

`kind` is a **positional** argument — `docker`, `nginx`, or `lambda`, not a `--kind` flag.

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

## 4. Deploy to AWS Lambda (optional)

```sh
cargo loco generate deployment lambda
```

Loco builds a standard Axum `Router`, and both Axum and the AWS Lambda runtime are `tower::Service`s — so your app runs on Lambda with no rewrite. This writes:

- `src/bin/lambda.rs` — a Lambda entrypoint that boots your app in `ServerOnly` mode and hands the router to the Lambda HTTP runtime (`lambda_http::run`). It's a separate binary target, so `cargo loco start` and your CLI are untouched.
- adds `lambda_http` to your `Cargo.toml`.
- writes a `[package.metadata.lambda]` block to your `Cargo.toml` so build/deploy need **no extra flags** — it declares which runtime files ship in the zip (`config/`, plus `assets/` etc. when detected) and sensible deploy defaults (`memory`, `timeout`, `LOCO_ENV=production`). Nothing environment-specific is baked in — region, account, IAM role and secrets are supplied at deploy time.

Then deploy with [cargo-lambda](https://www.cargo-lambda.info) — two commands, no flags:

```sh
cargo install cargo-lambda
cargo lambda build --release --arm64 --output-format zip
cargo lambda deploy --enable-function-url
```

`deploy` creates the function, an execution role, and a public **Function URL**, then prints the HTTPS endpoint. Set secrets with `cargo lambda deploy --enable-function-url --env-var DATABASE_URL=... --env-var JWT_SECRET=...` (or wire Secrets Manager).

### The deliverable

`cargo lambda build` produces a `.zip` under `target/lambda/lambda/` containing a single `bootstrap` executable (your compiled Rust binary) plus the runtime files declared in the metadata `include`. That zip is the *entire* artifact submitted to AWS — there's no managed runtime layer; it runs on the `provided.al2023` custom runtime. **Measured for a stock db app:** ~18 MB unzipped → **~8 MB zip** — well under Lambda's 50 MB zipped / 250 MB unzipped direct-upload limit, so no S3 staging. Prefer `--arm64` (Graviton) for lower cost and faster cold starts.

**What ships beyond the binary:** anything Loco reads from disk at runtime — always `config/`, plus `assets/`, i18n files, and `src/mailers/` templates if your app serves views/static assets/i18n/mail. The generator detects these and lists them in the metadata `include`; add more entries there if you read other files at runtime. (Alternatively, containerize — see below — which packages the whole app dir.)

### What this touches on the AWS side

Beyond the function itself, a working deploy involves:

- **A front door.** A [Lambda Function URL](https://docs.aws.amazon.com/lambda/latest/dg/urls-configuration.html) or an API Gateway (v2 HTTP API). `lambda_http` handles all three event shapes (Function URL, API GW v1/v2) transparently.
- **An IAM execution role.** CloudWatch Logs permissions at minimum (`cargo lambda deploy` can create a basic role, or pass `--role`); add VPC-access permissions if you attach to a VPC, plus permissions for anything the app calls (S3, SES, Secrets Manager).
- **Logs.** Loco's tracing output goes to stdout → CloudWatch Logs. Use JSON logging in production.
- **Config & secrets.** Set `LOCO_ENV` and secrets (`DATABASE_URL`, JWT secret, …) as function env vars (`--env-var`) or via Secrets Manager/SSM.
- **Networking to a database.** To reach RDS in a VPC, attach the function to the VPC's subnets + a security group (needs the VPC-access role). Outbound calls to SES/S3/Secrets Manager then need a NAT gateway or VPC endpoints. **Strongly consider [RDS Proxy](https://docs.aws.amazon.com/lambda/latest/dg/services-rds-tutorial.html):** each warm Lambda instance holds its own DB connections, so scaling can exhaust Postgres connection limits — RDS Proxy pools them.

### Notes & limits

- **HTTP only.** Background workers and the scheduler aren't started in the Lambda entrypoint — they don't fit Lambda's request/response model. Run those on an always-on target (ECS/EC2), or drive them from SQS/EventBridge.
- **Keep migrations out of the request path.** Run `cargo loco db migrate` from CI or a one-off task, and point the runtime `LOCO_ENV` at a config that doesn't auto-migrate on boot.
- **Cold starts.** Rust starts fast, but Loco boots the whole app (including DB connect) per cold start; VPC attachment adds ENI setup latency. Provisioned concurrency smooths this if needed.
- **Prefer zero code changes?** You can instead containerize your normal binary with the [AWS Lambda Web Adapter](https://github.com/awslabs/aws-lambda-web-adapter) on top of the generated `Dockerfile` — no `lambda.rs` needed, and the whole app dir (config, assets) ships in the image.

## 5. Set the environment variables production requires

There's no separate "production mode" — Loco picks a config file by environment, and **the default environment is `development`**. Set `LOCO_ENV=production` on the server (or pass `--environment production`), or your app will read `config/development.yaml` there and appear to work.

`config/production.yaml` was written for you at `loco new`, already tuned for production: backtraces off, `json` logs, `0.0.0.0` binding, a real connection pool.

What it deliberately does **not** contain is any secret or address. Those read from the environment with no fallback:

| Variable | Used for | Required |
| --- | --- | --- |
| `DATABASE_URL` | Database connection | yes, with a database |
| `JWT_SECRET` | Signing and verifying tokens | yes, with auth |
| `HOST` | The public URL that links in outgoing mail point to | yes |
| `MAILER_HOST`, `MAILER_USER`, `MAILER_PASSWORD` | SMTP server and credentials | yes, with a mailer |
| `REDIS_URL` / `QUEUE_URL` | Queue backend | yes, with a queue |
| `PORT`, `BINDING`, `LOG_LEVEL`, `DB_MAX_CONNECTIONS`, `DB_AUTO_MIGRATE` | Overrides | no, sensible defaults |

A missing required variable stops the app at startup with the variable's name, rather than letting it run with a development secret or point at a database that isn't there. That is intentional: a boot failure you can read is better than an app that appears healthy and is signing tokens with a key committed to your repository.

```sh
export DATABASE_URL='postgres://user:password@db-host:5432/myapp_production'
export JWT_SECRET="$(openssl rand -hex 32)"
export HOST='https://myapp.example.com'
```

If you run more than one instance, or migrate as a separate release step, set `DB_AUTO_MIGRATE=false` so instances don't race to migrate the same database.

See [Configure logging](/docs/how-to/configure-logging) for logging, and the [Configuration reference](/docs/reference/configuration) for every key in the file.

## 6. Run `loco doctor` before going live

```sh
myapp-cli doctor --environment production
```

Run this on the server, where the environment variables and the database it will actually use are. `doctor` opens the production config, connects to the DB and queue it names, and additionally reports settings that are safe in development and not in production — a loopback binding, `dangerously_truncate`, backtraces left on.

Add `-c`/`--config` to print the fully-resolved config, after environment substitution, for inspection:

```sh
myapp-cli doctor --config --environment production
```

`--production` still works as a deprecated alias for `--environment production`.

## 7. Ship it

Copy the binary and the `config/` folder to the server (no source, no `Cargo.lock`, no toolchain needed):

```sh
scp target/release/myapp-cli config/ user@server:/opt/myapp/
ssh user@server 'LOCO_ENV=production /opt/myapp/myapp-cli start'
```

## Verify

- `myapp-cli doctor --environment production` exits 0 and reports all checks passing.
- `myapp-cli start` boots and the startup banner shows the environment, DB, and logger you expect.
- Hitting the app's health/root route through nginx (if you generated one) returns a response, confirming the reverse proxy is wired to the right host/port.

## Reference

- `generate deployment` CLI shape (`docker`/`nginx`/`lambda` as `kind`): [CLI reference](/docs/reference/cli)
- Every config key referenced above (`logger`, `server`, `database`, `auth`, `mailer`, `queue`): [Configuration reference](/docs/reference/configuration)
