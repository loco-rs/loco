# Tasks

Think of _tasks_ as ad-hoc scripts only written in Rust instead of shell scripts, and having the full power of the entire `rustyrails` framework including access to database, models, mailers, and more.

You can use tasks to:

* Run a maintenance job on a database
* Ad hoc data fixing
* A periodic job, that you need to plug a cron entry for, such as sending a birthday email to users
* Produce and email reports

And more.

## Adding tasks

Tasks go in `src/tasks`, to add one, add a file, here `user_report.rs`:

```rust
#[async_trait]
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_report".to_string(),
            detail: "output a user report".to_string(),
        }
    }
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        // you can freely access the database, environment is loaded,
        // connection is established and ready
        // let users = users::Entity::find().all(&app_context.db).await?;
        println!("args: {vars:?}");
        println!("done: {} users", users.len());
        Ok(())
    }
}
```

Register it in your `app.rs`:

```rust
#[async_trait]
impl Hooks for App {
    //..
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::user_report::UserReport);
    }
    //..
}
```

## Using tasks

To use a task run it with `rr`:

```
$ rr task
(prints a list)
$ rr task user_report
```
