+++
title = "Schedule recurring jobs"
description = "Configure the scheduler to run a task or shell command on a cron or English-language schedule."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 22
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/processing/scheduler/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: run a [task](@/docs/how-to/write-task.md) or a shell command on a recurring schedule, without hand-rolling `crontab`.

## 1. Create a scheduler config

Generate a dedicated file:

```sh
cargo loco generate scheduler
```

This creates `config/scheduler.yaml`. Alternatively, add a `scheduler:` block directly to your environment YAML (`config/development.yaml`, etc.) — both forms use the same schema.

## 2. Define jobs

```yaml
scheduler:
  output: stdout # default output for all jobs: stdout | silent
  jobs:
    write_content:
      shell: true # run `run` as a shell command (default: false = run a task)
      run: "echo loco >> ./scheduler.txt"
      schedule: run every 1 second # English syntax
      output: silent # overrides the job-level default
      tags: ["base", "infra"]

    run_task:
      run: "foo" # a registered task name
      schedule: "at 10:00 am"
      run_on_start: true # also run once when the scheduler starts

    list_if_users:
      run: "user_report"
      shell: true
      schedule: "* 2 * * * *" # cron syntax
      tags: ["base", "users"]
```

Each job entry has:

| Key | Required? | Notes |
|---|---|---|
| `run` | yes | A shell command (if `shell: true`) or a registered task name plus optional `KEY:VALUE` args (if `shell: false`, the default) |
| `schedule` | yes | English phrase or cron expression — see below |
| `shell` | no, default `false` | `false` runs `run` as a task; `true` runs it as a shell command |
| `run_on_start` | no, default `false` | Also fire once immediately when the scheduler starts |
| `tags` | no | Group jobs so you can run them together with `--tag` |
| `output` | no | Overrides `scheduler.output` for this job only |

### Schedule syntax

`schedule` accepts either form — Loco auto-detects cron syntax by checking whether the string starts with a digit or `*`; anything else is parsed as English via `english_to_cron`:

- English: `every 15 seconds`, `run every minute`, `fire every day at 4:00 pm`, `at 10:00 am`, `run at midnight on the 1st and 15th of the month`, `On Sunday at 12:00`, `7pm every Thursday`, `midnight on Tuesdays`
- Cron (7 fields, **UTC**, includes seconds and year):

  ```
  sec   min   hour   day of month   month   day of week   year
  *     *     *      *              *       *             *
  ```

## 3. Verify the config

```sh
# dedicated file
cargo loco scheduler --config config/scheduler.yaml --list

# scheduler: block embedded in the environment file
LOCO_ENV=production cargo loco scheduler --list
```

## 4. Run it

As a standalone process:

```sh
cargo loco scheduler                                    # uses scheduler: in config/<env>.yaml
cargo loco scheduler --config config/scheduler.yaml      # uses a dedicated file
```

Or bundled with the server and worker in one process:

```sh
cargo loco start --all
```

If your jobs live in a dedicated `scheduler.yaml` rather than embedded in the environment file, `start --all` needs to be told where to find it — set `SCHEDULER_CONFIG`:

```sh
SCHEDULER_CONFIG=config/scheduler.yaml cargo loco start --all
```

Each firing spawns a **subprocess** (`/bin/sh -c` on Unix, `cmd.exe /C` on Windows); `LOCO_ENV` is propagated to it, so a task job resolves the same config/environment as the parent process. On shutdown (Ctrl+C), the scheduler waits for running jobs before exiting.

## 5. Run a subset by name or tag

```sh
LOCO_ENV=production cargo loco scheduler --name 'run_task'
LOCO_ENV=production cargo loco scheduler --tag 'base'
```

## Reference

- Writing the task a scheduler job invokes: [Write a task](@/docs/how-to/write-task.md)
- `scheduler`/`SCHEDULER_CONFIG` config keys: [Configuration reference](@/docs/reference/configuration.md)
- `cargo loco scheduler` flags: [CLI reference](@/docs/reference/cli.md)
