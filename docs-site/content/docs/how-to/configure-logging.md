+++
title = "Configure logging"
description = "Set logger level and format, understand the filtering precedence, and add a rotating file appender."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 33
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

Goal: control what Loco logs, in what shape, and where — stdout for development, structured JSON for production, and (optionally) a rotating log file — without drowning in third-party crate noise.

Loco's logger is built on `tracing`. `logger.enable`, `logger.level`, and `logger.format` are required keys in every config file.

## 1. Set the minimum config

```yaml
# config/development.yaml
logger:
  enable: true
  pretty_backtrace: true
  level: debug
  format: compact
```

- `level`: `off` | `trace` | `debug` | `info` | `warn` | `error`.
- `format`: `compact` | `pretty` | `json`.
- `pretty_backtrace`: when `true`, forces `RUST_BACKTRACE=1` and nicely-formatted panic backtraces. It's a development convenience — turn it off in performance-sensitive production deployments.

## 2. Know the filtering precedence

Loco doesn't just apply `level` globally to every crate — by default it whitelists a small set of modules (`loco_rs`, `sea_orm_migration`, `tower_http`, `sqlx::query`, `playground`, `loco_gen`) plus your own app crate, and applies `level` only to those. Everything else stays quiet.

Three ways to control this, in strict precedence order:

1. **`RUST_LOG` environment variable** — if set, it wins outright, ignoring both `level` and `override_filter`. Use this for one-off debugging on a running process without touching config:
   ```sh
   RUST_LOG=debug cargo loco start
   ```
2. **`logger.override_filter`** — a raw `tracing-subscriber` `EnvFilter` directive string. Use this to permanently see traces from libraries outside the built-in whitelist:
   ```yaml
   logger:
     enable: true
     level: info
     format: compact
     override_filter: "trace" # or a directive like "myapp=debug,tower_http=debug"
   ```
3. **The built-in module whitelist + `level`** — what you get if neither of the above is set. This is the common case: set `level` and trust Loco's whitelist to keep noise down.

## 3. Choose a format per environment

- `compact` — human-readable single-line output; good default for local development.
- `pretty` — multi-line, more spacious human-readable output.
- `json` — structured, one JSON object per line; use this in production so a log aggregator (Loki, CloudWatch, Datadog, etc.) can parse fields directly.

```yaml
# config/production.yaml
logger:
  enable: true
  pretty_backtrace: false
  level: info
  format: json
```

## 4. Add a rotating file appender

`file_appender` writes logs to disk independently of (and with its own level/format, separate from) the stdout logger — useful when you want stdout kept quiet but still capture a full trail on disk.

```yaml
logger:
  enable: true
  level: info
  format: compact
  file_appender:
    enable: true
    non_blocking: false   # true offloads writes to a background thread
    level: debug
    format: json
    rotation: daily       # minutely | hourly | daily | never — default is hourly
    dir: ./logs           # default "./logs" if omitted
    filename_prefix: myapp
    filename_suffix: log
    max_log_files: 7      # required — old files beyond this count are pruned
```

With `rotation: daily` and the settings above, you'll get files like `./logs/myapp.<date>.log`, rotated once a day, with only the newest 7 kept around.

## 5. Verify

Start the app and confirm the shape you expect shows up:

```sh
cargo loco start
# ... watch stdout for compact/pretty/json-formatted lines at the level you set
```

If you enabled a file appender, tail the log directory:

```sh
tail -f ./logs/*.log
```

To confirm the filtering precedence, try overriding at runtime without touching the config file:

```sh
RUST_LOG=loco_rs=trace cargo loco start
```

You should see much more verbose output than `logger.level` alone would produce — confirming `RUST_LOG` took precedence.

## Reference

- Every `logger:` YAML key, including all `file_appender` sub-keys: [Configuration reference § logger](@/docs/reference/configuration.md#logger)
