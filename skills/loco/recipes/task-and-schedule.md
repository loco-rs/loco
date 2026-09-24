# Recipe: tasks and the scheduler

This is the recipe agents most often get wrong, because the three mechanisms
look interchangeable and are not. Loco copies Rails' answer exactly.

| Loco | Rails ancestor | Triggered by | Use when |
|---|---|---|---|
| **`Task`** | `rake task` | a human at a CLI | one-off / operational work |
| **`BackgroundWorker`** | Active Job | your code, mid-request | the request must return first |
| **Scheduler** | `whenever` / cron | the clock | recurring on a wall-clock schedule |

**The scheduler does not contain work.** Its config holds a `run:` string that
is "a task name and also task arguments," and it executes it as a subprocess.
So *recurring work is always two pieces*: a `Task` that does the thing, and a
scheduler entry that runs that task on a cron expression — exactly like
`whenever` generating a crontab that calls `rake`.

If you are writing a scheduler entry and find yourself wanting to put logic in
it, you want a task.

## Write a task

```sh
cargo loco generate task cleanup_sessions
```

```rust
use loco_rs::prelude::*;

pub struct CleanupSessions;

#[async_trait]
impl Task for CleanupSessions {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "cleanup_sessions".to_string(),
            detail: "Delete sessions older than the retention window".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        // Optional CLI arguments: `cargo loco task cleanup_sessions days:30`
        let days: i64 = vars.cli_arg("days").unwrap_or("30").parse().unwrap_or(30);

        let removed = sessions::Model::purge_older_than(&ctx.db, days).await?;
        tracing::info!(removed, days, "cleaned up sessions");
        Ok(())
    }
}
```

The generator registers it in `Hooks::register_tasks` in `src/app.rs`:

```rust
fn register_tasks(tasks: &mut Tasks) {
    tasks.register(tasks::cleanup_sessions::CleanupSessions);
}
```

Run it:

```sh
cargo loco task                              # list registered tasks
cargo loco task cleanup_sessions             # run it
cargo loco task cleanup_sessions days:7      # with arguments
```

`vars.cli_arg(key)` returns `Result<&str>` — it errors when the key is absent,
so use `.unwrap_or(...)` for optional arguments.

The task gets the full `AppContext`: `ctx.db`, `ctx.config`, `ctx.storage`,
`ctx.mailer`, `ctx.cache`. It runs in its own process and exits.

## Schedule it

**Scheduler jobs live under a `scheduler:` key in `config/<env>.yaml`.** That is
where the scheduler looks by default — `Config::scheduler` is an
`Option<scheduler::Config>`.

```yaml
# config/development.yaml
scheduler:
  output: silent
  jobs:
    cleanup_sessions:
      run: "cleanup_sessions days:30"
      schedule: "0 0 3 * * *"
```

A standalone file is also supported, but only when you point at it explicitly:

```sh
cargo loco generate scheduler                    # writes config/scheduler.yaml
cargo loco scheduler --config config/scheduler.yaml --list
```

Writing `config/scheduler.yaml` and running plain `cargo loco scheduler --list`
gives `Scheduler(Empty)` — the file is never read. This is the most common
scheduler mistake.

The job block itself is the same either way:

```yaml
output: silent
jobs:
  cleanup_sessions:
    run: "cleanup_sessions days:30"
    schedule: "0 0 3 * * *"
    tags: ["maintenance"]

  send_digest:
    run: "send_digest"
    schedule: "0 0 8 * * MON"
    output: stdout

  vacuum:
    run: "psql -c 'VACUUM ANALYZE'"
    shell: true
    schedule: "0 30 4 * * SUN"
```

Fields on a job:

| Field | Meaning |
|---|---|
| `run` | a task name plus arguments — or, with `shell: true`, a shell command |
| `schedule` | a cron expression, **or plain English** (see below) |
| `shell` | run through a shell instead of invoking a task (default `false`) |
| `run_on_start` | also run once immediately at scheduler startup |
| `tags` | filter which jobs a given scheduler process runs |
| `output` | `silent` or `stdout`; overrides the top-level default |

**The cron expression starts with seconds**, not minutes:
`sec min hour day-of-month month day-of-week year`. `0 0 3 * * *` is 3:00:00 AM
daily. A 5-field crontab string copied from elsewhere will be misinterpreted.

`schedule` also accepts plain English, which Loco converts via
`english-to-cron` — often clearer, and it sidesteps the seconds-field trap
entirely:

```yaml
schedule: "every day at 3:00 am"
schedule: "run every 1 second"
schedule: "at 10:00 am"
```

Inspect and run:

```sh
cargo loco scheduler --list                  # show the parsed schedule
cargo loco scheduler --name cleanup_sessions # run one job now
cargo loco scheduler --tag maintenance       # run only tagged jobs
cargo loco scheduler                         # start the scheduler
```

## Choosing, in one pass

- The user is waiting on an HTTP response → **worker** (`perform_later`).
- A human will run it by hand → **task**.
- It must happen every night → **task** + a scheduler entry.
- It must happen every night *and it is slow* → still a task + scheduler entry.
  The scheduler runs it in its own process; that is already out-of-band.
- You are reaching for `tokio::spawn` → you want a **worker**.
