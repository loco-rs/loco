+++
title = "Worker"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 22
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

You can easily test your worker background jobs using `Loco`. Ensure that your worker is set to the `ForegroundBlocking` mode, which blocks the job, ensuring it runs synchronously. When testing the worker, the test will wait until your worker is completed, allowing you to verify if the worker accomplished its intended tasks.

It's recommended to implement tests in the `tests/workers` directory to consolidate all your worker tests in one place.

Additionally, you can leverage the [worker generator](@/docs/the-app/workers.md#generate-a-worker), which automatically creates tests, saving you time on configuring tests in the library.

Here's an example of how the test should be structured:

```rust
#[tokio::test]
#[serial]
async fn test_run_report_worker_worker() {
    // Set up the test environment
    let boot = testing::boot_test::<App, Migrator>().await.unwrap();

    // Execute the worker in 'ForegroundBlocking' mode, preventing it from running asynchronously
    assert!(
        ReportWorkerWorker::perform_later(&boot.app_context, ReportWorkerWorkerArgs {})
            .await
            .is_ok()
    );

    // Include additional assert validations after the execution of the worker
}

```
