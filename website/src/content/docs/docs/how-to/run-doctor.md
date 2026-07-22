---
title: Diagnose your app with cargo loco doctor
description: Run `cargo loco doctor` to validate DB/queue connectivity, dependency versions, and initializer health, with the --config and --production flags.
sidebar:
  order: 62
---

Goal: quickly check whether your app's environment (database, queue, tooling, dependency versions) is set up correctly, both locally and in CI.

## 1. Run it

```sh
cargo loco doctor
```

Each check prints one line with a status icon, and a description on the line(s) below when there's something actionable to say:

```
✅ DB connection: success
✅ queue connection: success
❌ SeaORM CLI was not found
   To fix, run:
      $ cargo install sea-orm-cli
```

- ✅ = `Ok`
- ❌ = `NotOk`
- ⚠️ = `NotConfigure` (the resource isn't configured — not necessarily an error)

If **any** check comes back `NotOk`, the process exits with a non-zero status — wire `cargo loco doctor` into CI to fail the build on a broken DB/queue connection or an outdated dependency.

## 2. What gets checked

`doctor` always runs:

| Check | Condition | What it does |
|---|---|---|
| Database | `with-db` enabled | Connects, pings, and verifies access using `config.database`. |
| Queue | `workers.mode` is `BackgroundQueue` | Creates the queue provider and pings it; reports `NotConfigure` if no queue is set up. |
| Initializer checks | any registered `Initializer` implements `check()` | Runs each one, prefixing its message with `Initializer {name}: `. |

...and, **only when not run with `--production`**, three more:

| Check | What it does |
|---|---|
| Deps | Reads `Cargo.lock` and flags any "blessed" dependency below its minimum version. |
| SeaOrmCLI | Runs `sea-orm-cli --version` and checks it against the minimum. |
| PublishedLocoVersion | Compares your `loco-rs` version against what's published on crates.io. |

Current blessed minimum versions: `tokio 1.33.0`, `sea-orm 2.0.0-rc`, `validator 0.20.0`, `axum 0.8.1`. (`sea-orm`/`sea-orm-cli` are still pinned to the `2.0.0-rc` line as of this writing — expect that floor to move to `2.0.0` once Sea-ORM ships stable.)

## 3. Skip dev-only checks in production with `--production`

Deployed environments typically don't have `sea-orm-cli` installed, may not have network access to crates.io, and don't need a "you're behind on X" nag on every boot. `--production` (short `-p`) skips the three dev-only checks above and only runs Database/Queue/Initializer checks:

```sh
cargo loco doctor --production
```

## 4. Inspect resolved configuration with `--config`

`--config` (short `-c`) bypasses checks entirely and instead dumps the fully-resolved `Config` (as YAML) plus the active environment name — useful when you're not sure which config file/environment actually got loaded:

```sh
cargo loco doctor --config
```

```
# ...your full resolved config, dumped as YAML...
Environment: development
```

This complements the [configuration reference](/docs/reference/configuration#loading-precedence) — if a setting doesn't look like what you expect, `doctor --config` shows you the config *after* file precedence, environment resolution, and Tera templating have all been applied, not just what's on disk.

## 5. Add your own checks

If you ship a custom [`Initializer`](/docs/how-to/add-middleware), implement its `check` method and `doctor` will pick it up automatically — no extra wiring needed:

```rust
use loco_rs::doctor::{Check, CheckStatus};

async fn check(&self, app_context: &AppContext) -> loco_rs::Result<Option<Check>> {
    // return None to opt out, or Some(Check { .. }) to report a result
    Ok(Some(Check {
        status: CheckStatus::Ok,
        message: "connected".to_string(),
        description: None,
    }))
}
```

## Verify it

```sh
cargo loco doctor
echo $?           # 0 if every check passed, non-zero if any check is NotOk
cargo loco doctor --production
cargo loco doctor --config
```

See the [CLI reference](/docs/reference/cli#2-1-top-level-subcommands) for the exact flag list alongside every other `cargo loco` subcommand.
