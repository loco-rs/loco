+++
title = "Workers"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

Loco provides the following options for background jobs:

* Redis backed (powered by `sidekiq-rs`)
* Postgres backed (own implementation)
* SQLite backed (own implementation)
* Tokio-async based (same-process, evented thread based background jobs)


You enqueue and perform jobs without knowledge of the actual background queue implementation, similar to  Rails' _ActiveJob_, so you can switch with a simple change of configuration and no code change.

## Async vs Queue

When you generated a new app, you might have selected the default `async` configuration for workers. This means workers spin off jobs in Tokio's async pool, which gives you proper background processes in the same running server.

You might want to configure jobs to run in a separate process backed by a queue, in order to distribute the load across servers.

First, switch to `BackgroundQueue`:

```yaml
# Worker Configuration
workers:
  # specifies the worker mode. Options:
  #   - BackgroundQueue - Workers operate asynchronously in the background, processing queued.
  #   - ForegroundBlocking - Workers operate in the foreground and block until tasks are completed.
  #   - BackgroundAsync - Workers operate asynchronously in the background, processing tasks with async capabilities.
  mode: BackgroundQueue
```

Then, configure a Redis based queue backend:

```yaml
queue:
  kind: Redis
  # Redis connection URI.
  uri: "{{ get_env(name="REDIS_URL", default="redis://127.0.0.1") }}"
  # Dangerously flush all data.
  dangerously_flush: false
  # represents the number of tasks a worker can handle simultaneously.
  num_workers: 2
```

Or a Postgres based queue backend:

```yaml
queue:
  kind: Postgres
  # Postgres Queue connection URI.
  uri: "{{ get_env(name="PGQ_URL", default="postgres://localhost:5432/mydb") }}"
  # Dangerously flush all data.
  dangerously_flush: false
  # represents the number of tasks a worker can handle simultaneously.
  num_workers: 2
```

Or a SQLite based queue backend:

```yaml
queue:
  kind: Sqlite
  # SQLite Queue connection URI.
  uri: "{{ get_env(name="SQLTQ_URL", default="sqlite://loco_development.sqlite?
  mode=rwc") }}"
  # Dangerously flush all data. 
  dangerously_flush: false
  # represents the number of tasks a worker can handle simultaneously.
  num_workers: 2
```

## Running the worker process
You can run in two ways, depending on which setting you chose for background workers:

```
Usage: demo_app start [OPTIONS]

Options:
  -w, --worker                     start worker
  -s, --server-and-worker          start same-process server and worker
```

Choose `--worker` when you configured a real Redis queue and you want a process for doing just background jobs. You can use a single process per server. In this case, you can run your main Web or API server using just `cargo loco start`.

```sh
$ cargo loco start --worker # starts a standalone worker job executing process
$ cargo loco start # starts a standalone API service or Web server, no workers
```

Choose `-s` when you configured `async` background workers, and jobs will execute as part of the current running server process.

For example, running `--server-and-worker`:

```sh
$ cargo loco start --server-and-worker # both API service and workers will execute
```

## Creating background jobs in code

To use a worker, we mainly think about adding a job to the queue, so you `use` the worker and perform later:

```rust
    // .. in your controller ..
    DownloadWorker::perform_later(
        &ctx,
        DownloadWorkerArgs {
            user_guid: "foo".to_string(),
        },
    )
    .await
```

Unlike Rails and Ruby, with Rust you can enjoy _strongly typed_ job arguments which gets serialized and pushed into the queue.

### Using shared state from a worker

See [How to have global state](@/docs/the-app/controller.md#global-app-wide-state), but generally you use a single shared state by using something like `lazy_static` and then simply refer to it from the worker.

If this state can be serializable, _strongly prefer_ to pass it through the `WorkerArgs`.


## Creating a new worker

Adding a worker meaning coding the background job logic to take the _arguments_ and perform a job. We also need to let `loco` know about it and register it into the global job processor.

Add a worker to `workers/`:

```rust
#[async_trait]
impl BackgroundWorker<DownloadWorkerArgs> for DownloadWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, args: DownloadWorkerArgs) -> Result<()> {
        println!("================================================");
        println!("Sending payment report to user {}", args.user_guid);

        // TODO: Some actual work goes here...

        println!("================================================");
        Ok(())
    }
}
```

And register it in `app.rs`:

```rust
#[async_trait]
impl Hooks for App {
//..
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }
// ..
}
```

### Generate a Worker

To automatically add a worker using `loco generate`, execute the following command:

```sh
cargo loco generate worker report_worker
```

The worker generator creates a worker file associated with your app and generates a test template file, enabling you to verify your worker.

## Configuring Workers

In your `config/<environment>.yaml` you can specify the worker mode. BackgroundAsync and BackgroundQueue will process jobs in a non-blocking manner, while ForegroundBlocking will process jobs in a blocking manner.

The main difference between BackgroundAsync and BackgroundQueue is that the latter will use Redis/Postgres/SQLite to store the jobs, while the former does not require Redis/Postgres/SQLite and will use async in memory within the same process.

```yaml
# Worker Configuration
workers:
  # specifies the worker mode. Options:
  #   - BackgroundQueue - Workers operate asynchronously in the background, processing queued.
  #   - ForegroundBlocking - Workers operate in the foreground and block until tasks are completed.
  #   - BackgroundAsync - Workers operate asynchronously in the background, processing tasks with async capabilities.
  mode: BackgroundQueue
```

## Manage a Workers From UI
You can manage the jobs queue with the [Loco admin job project](https://github.com/loco-rs/admin-jobs).
![<img style="width:100%; max-width:640px" src="tour.png"/>](https://github.com/loco-rs/admin-jobs/raw/main/media/screenshot.png)

### Managing Job Queues via CLI

The job queue management feature provides a powerful and flexible way to handle the lifecycle of jobs in your application. It allows you to cancel, clean up, remove outdated jobs, export job details, and import jobs, ensuring efficient and organized job processing.

## Features Overview

- **Cancel Jobs**  
  Provides the ability to cancel specific jobs by name, updating their status to `cancelled`. This is useful for stopping jobs that are no longer needed, relevant, or if you want to prevent them from being processed when a bug is detected.  
- **Clean Up Jobs**  
  Enables the removal of jobs that have already been completed or cancelled. This helps maintain a clean and efficient job queue by eliminating unnecessary entries.  
- **Purge Outdated Jobs**  
  Allows you to delete jobs based on their age, measured in days. This is particularly useful for maintaining a lean job queue by removing older, irrelevant jobs.  
  **Note**: You can use the `--dump` option to export job details to a file, manually modify the job parameters in the exported file, and then use the `import` feature to reintroduce the updated jobs into the system.  
- **Export Job Details**  
  Supports exporting the details of all jobs to a specified location in file format. This feature is valuable for backups, audits, or further analysis.  
- **Import Jobs**  
  Facilitates importing jobs from external files, making it easy to restore or add new jobs to the system. This ensures seamless integration of external job data into your application's workflow.  

To access the job management commands, use the following CLI structure:
<!-- <snip id="jobs-help-command" inject_from="yaml" action="exec" template="sh"> -->
```sh
Managing jobs queue

Usage: demo_app-cli jobs [OPTIONS] <COMMAND>

Commands:
  cancel  Cancels jobs with the specified names, setting their status to `cancelled`
  tidy    Deletes jobs that are either completed or cancelled
  purge   Deletes jobs based on their age in days
  dump    Saves the details of all jobs to files in the specified folder
  import  Imports jobs from a file
  help    Print this message or the help of the given subcommand(s)

Options:
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- </snip> -->



## Testing a Worker

You can easily test your worker background jobs using `Loco`. Ensure that your worker is set to the `ForegroundBlocking` mode, which blocks the job, ensuring it runs synchronously. When testing the worker, the test will wait until your worker is completed, allowing you to verify if the worker accomplished its intended tasks.

It's recommended to implement tests in the `tests/workers` directory to consolidate all your worker tests in one place.

Additionally, you can leverage the [worker generator](@/docs/processing/workers.md#generate-a-worker), which automatically creates tests, saving you time on configuring tests in the library.

Here's an example of how the test should be structured:


```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn test_run_report_worker_worker() {
    // Set up the test environment
    let boot = boot_test::<App, Migrator>().await.unwrap();

    // Execute the worker in 'ForegroundBlocking' mode, preventing it from running asynchronously
    assert!(
        ReportWorkerWorker::perform_later(&boot.app_context, ReportWorkerWorkerArgs {})
            .await
            .is_ok()
    );

    // Include additional assert validations after the execution of the worker
}

```
