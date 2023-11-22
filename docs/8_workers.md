# Workers

`rustyrails` integrates with a full blown background job processing framework: `sidekiq-rs`. You can enqueue jobs in a similar ergonomics as Rails' _ActiveJob_, and have a similar scalable processing model to perform these background jobs.


## Using workers

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

## Adding a worker

Adding a worker meaning coding the background job logic to take the _arguments_ and perform a job. We also need to let `rustyrails` know about it and register it into the global job processor.

Add a worker to `workers/`:

```rust
#[async_trait]
impl Worker<DownloadWorkerArgs> for DownloadWorker {
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
    fn connect_workers<'a>(p: &'a mut Processor, ctx: &'a AppContext) {
        p.register(DownloadWorker::build(ctx));
    }
// ..    
}
```

