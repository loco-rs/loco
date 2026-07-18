---
title: The configuration model
description: How Loco resolves an environment, which config file wins, why YAML is rendered through Tera first, and how secrets flow in without a dedicated vault type.
sidebar:
  order: 4
---

Every subsystem described elsewhere in this cluster — the DB pool, the queue backend, the cache, the mailer, the middleware stack — is switched on and tuned from one place: a per-environment YAML file, deserialized into one typed `Config` struct. This page explains the small number of rules that govern how that file is found, rendered, and trusted with secrets. For the exhaustive key-by-key listing, see the [Configuration reference](/docs/reference/configuration).

## Why a typed config struct, not ad-hoc env vars

The alternative most hand-rolled Axum services fall into is reading a scatter of environment variables (`DATABASE_URL`, `PORT`, `RUST_LOG`, ...) directly in `main()`, each parsed and defaulted slightly differently, with no single place that shows what the app's full configuration surface even is. Loco instead deserializes the whole environment file into one `Config` struct (`src/config/mod.rs`), with every sub-area — `server`, `database`, `logger`, `queue`, `cache`, `mailer`, `auth`, `workers` — as a typed field with `serde` defaults where a sane one exists and a hard requirement (a missing-field deserialize error at boot) where there isn't one. This is the same "prefer a built-in over hand-wiring" bias covered in [Why batteries included](/docs/explanation/why-batteries-included), applied specifically to app configuration: you get one document that *is* the app's configuration surface, checked at boot rather than discovered at the call site that happens to read an env var.

## Which environment, and which file

Two independent questions get resolved before any YAML is even opened:

**Which environment name?** `environment::resolve_from_env()` checks, in order:

1. `LOCO_ENV`
2. `RAILS_ENV`
3. `NODE_ENV`
4. falls back to `"development"`

The `RAILS_ENV`/`NODE_ENV` fallbacks exist so that a Loco app dropped into infrastructure already standardized on Rails- or Node-style environment naming doesn't need a separate variable just for Loco.

**Which file, for that environment name?** `Config::from_folder` picks the *first* file that exists, in this order:

1. `{env}.local.yaml`
2. `{env}.yaml`

If neither exists, boot fails outright with "no configuration file found." The `.local.yaml` tier is the mechanism for machine-local overrides — a developer's own DB credentials, a locally-running service's port — that should never be checked into version control alongside the shared `{env}.yaml`. It's a convention, not a special format: `development.local.yaml` is parsed with exactly the same rules as `development.yaml`, it's just consulted first and expected to be gitignored.

Both tiers are read from a `config/` folder by default; `LOCO_CONFIG_FOLDER` overrides that location, which matters for deployments that mount configuration from somewhere other than the app's own source tree.

## The YAML is a Tera template first, a config file second

Before `serde_yaml` ever sees the file, its entire contents are rendered as a [Tera](https://keats.github.io/tera/) template (`Tera::one_off(.., autoescape = false)`). This is a small design choice with a real consequence: it's what makes patterns like this legal inside a Loco config file at all —

```yaml
server:
  port: {{ get_env(name="NODE_PORT", default=5150) }}
```

`get_env(name=.., default=..)` here is **Tera's own built-in function**, not something Loco registers. Loco doesn't have a custom templating layer bolted onto YAML — it reuses a general-purpose template engine's existing capability (reading env vars, with a default) so that config files can be static-looking YAML *and* environment-aware at the same time, without inventing a second interpolation syntax. Anything else Tera can do in a one-off render (conditionals, other built-in functions) is available in a config file too, though `get_env` covers the overwhelming majority of real use.

The practical implication: a config value that looks hardcoded may not be — always check for `{{ }}` before assuming a YAML value is literal — and a value that needs to differ between "what's checked into git" and "what's true on this machine/host" belongs behind `get_env`, not behind a second config file.

## Secrets: a convention, not a vault type

There is no dedicated `Secret` type or vault integration built into `Config`. A JWT secret, an SMTP password, a database URI — these are all just `String` fields on ordinary config structs (`auth.jwt.secret`, `mailer.smtp.auth.password`, `database.uri`). The secrets *model* is the composition of two things already covered above:

- Secret values are injected via `get_env(name=..)` at render time, so the checked-in YAML never contains the literal secret — only the name of the environment variable to read it from.
- Machine-local secrets that shouldn't even have an env-var name in shared code can go in `{env}.local.yaml` instead, which is expected to be gitignored entirely.

This is deliberately unopinionated about *where* the environment variable itself comes from — a `.env` file, a process manager, a secrets manager injecting env vars at container start — because that's an operational concern outside the framework's scope, and Loco's contract stops at "a `String` field, populated from `get_env` or a local override file." One consequence worth knowing in advance: `auth.jwt.secret` specifically is expected to be valid **base64** (it's fed to `jsonwebtoken`'s `from_base64_secret` constructors) — a plain passphrase string will fail at the point the JWT extractor tries to decode it, not at config-load time.

## What this buys you day to day

Put together, the model gives you: one typed document per environment describing the whole app, a predictable override tier for anything machine-specific, and a templating escape hatch for anything environment-dependent — all without a second configuration DSL or a runtime service to stand up just to manage config. Changing a pool size, flipping a middleware on, or pointing at a different queue backend (see [The background-processing model](/docs/explanation/background-processing-model)) is a YAML edit and a restart, not a recompile — the same "config, not code" theme that runs through the rest of Loco's built-ins. See the [Configuration reference](/docs/reference/configuration) for every key, type, and default across every sub-config struct.
