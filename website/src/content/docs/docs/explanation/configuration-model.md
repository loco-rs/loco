---
title: The configuration model
description: How Loco resolves an environment, which config file wins, why YAML is rendered through a template pass first, and how secrets flow in without a dedicated vault type.
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

**Which file, for that environment name?** `Config::from_folder` reads both tiers and layers them:

1. `{env}.yaml` — the base, checked into version control
2. `{env}.local.yaml` — an optional override, expected to be gitignored

When both exist they are *deep-merged*, not chosen between: the two documents are walked key by key and the local value wins wherever the two collide, so a local file only needs to restate the handful of keys it actually changes rather than duplicate the whole document. The recursion stops at anything that isn't a mapping — a scalar or a sequence in the local file replaces its counterpart outright, so an overridden list is the local list, never the two concatenated. When only one of the two exists, that file is used on its own; if neither does, boot fails outright with "no configuration file found."

That makes `.local.yaml` the mechanism for machine-local overrides — a developer's own DB credentials, a locally-running service's port — without forking the shared file to get them. It's a convention, not a special format: `development.local.yaml` is parsed with exactly the same rules as `development.yaml`, it just gets the last word.

Both tiers are read from a `config/` folder by default; `LOCO_CONFIG_FOLDER` overrides that location, which matters for deployments that mount configuration from somewhere other than the app's own source tree.

## The YAML is templated first, a config file second

Before `serde_yaml` ever sees the file, its entire contents are rendered as a template. This is a small design choice with a real consequence: it's what makes patterns like this legal inside a Loco config file at all —

```yaml
server:
  port: <%= get_env(name="NODE_PORT", default="5150") %>
```

Three tag forms are recognized, each rendered before YAML parsing ever runs:

- `<%= expr %>` — interpolate a value (the common case, and the only one most configs need)
- `<% stmt %>` — a statement or block (conditionals, loops)
- `<%# text %>` — a comment, stripped from the rendered output entirely

Rendering is purely textual: `<%= get_env(name="PORT", default="5150") %>` renders to the bare characters `5150`, and only then does `serde_yaml` parse the result and type it as an integer. A boolean or numeric field stays a boolean or numeric field — there's no need to quote a templated value just because it's templated.

Under the hood, these tags still run through [Tera](https://keats.github.io/tera/): Loco translates the `<% %>` delimiters into Tera's native ones and renders the result. It can't use the obvious `Tera::one_off` shortcut, because that renders against a throwaway instance carrying only Tera's own built-ins — and `get_env(name=.., default=..)` is no longer one of them. Tera 1 shipped it; Tera 2 dropped it, leaving only `range` and `throw`. Since every Loco config depends on `get_env`, Loco registers its own implementation with Tera 1's semantics (return the variable if set, fall back to `default` if given, error if neither), which requires an instance it owns. Anything else Tera can do is available in a config file too, though `get_env` covers the overwhelming majority of real use.

### Why `<% %>` and not `{{ }}`

Tera's own delimiters are `{{ }}`/`{% %}`, and earlier Loco versions used them directly. The problem: `{` is a YAML flow-mapping indicator, so `port: {{ get_env(...) }}` is not valid YAML *at rest* — it only ever worked because Loco's template pass rewrote the file before anything else got a chance to parse it as YAML. Any tool that reads the file as YAML first — prettier, yaml-language-server, an editor's format-on-save — sees `{{ ... }}` as two nested flow mappings and "normalizes" it into `{ { ... } }`, which breaks app startup. That's [loco-rs/loco#1727](https://github.com/loco-rs/loco/issues/1727).

`<` is not a YAML indicator character, so `<%= ... %>` is an ordinary plain string scalar: the file parses as valid YAML *before* it's ever templated, and a formatter has nothing in it to restructure. This is the same trick Rails' ERB-in-`database.yml` has relied on for two decades.

The legacy `{{ }}`/`{% %}` form still renders — this is not a breaking change, and existing config files keep working untouched — but using it now logs a deprecation warning, and it should be migrated to `<% %>` wherever a file might pass through a formatter (in practice, that's most places).

The practical implication: a config value that looks hardcoded may not be — always check for `<% %>` (or the legacy `{{ }}`) before assuming a YAML value is literal — and a value that needs to differ between "what's checked into git" and "what's true on this machine/host" belongs behind `get_env`, not behind a second config file.

## Secrets: a convention, not a vault type

There is no dedicated `Secret` type or vault integration built into `Config`. A JWT secret, an SMTP password, a database URI — these are all just `String` fields on ordinary config structs (`auth.jwt.secret`, `mailer.smtp.auth.password`, `database.uri`). The secrets *model* is the composition of two things already covered above:

- Secret values are injected via `get_env(name=..)` at render time, so the checked-in YAML never contains the literal secret — only the name of the environment variable to read it from.
- Machine-local secrets that shouldn't even have an env-var name in shared code can go in `{env}.local.yaml` instead, which is expected to be gitignored entirely.

This is deliberately unopinionated about *where* the environment variable itself comes from — a `.env` file, a process manager, a secrets manager injecting env vars at container start — because that's an operational concern outside the framework's scope, and Loco's contract stops at "a `String` field, populated from `get_env` or a local override file." One consequence worth knowing in advance: `auth.jwt.secret` specifically is expected to be valid **base64** (it's fed to `jsonwebtoken`'s `from_base64_secret` constructors) — a plain passphrase string will fail at the point the JWT extractor tries to decode it, not at config-load time.

## What this buys you day to day

Put together, the model gives you: one typed document per environment describing the whole app, a predictable override tier for anything machine-specific, and a templating escape hatch for anything environment-dependent — all without a second configuration DSL or a runtime service to stand up just to manage config. Changing a pool size, flipping a middleware on, or pointing at a different queue backend (see [The background-processing model](/docs/explanation/background-processing-model)) is a YAML edit and a restart, not a recompile — the same "config, not code" theme that runs through the rest of Loco's built-ins. See the [Configuration reference](/docs/reference/configuration) for every key, type, and default across every sub-config struct.
