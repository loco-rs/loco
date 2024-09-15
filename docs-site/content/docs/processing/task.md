+++
title = "Tasks"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

Tasks in `Loco` serve as ad-hoc functionalities to handle specific aspects of your application. Whether you need to fix data, send emails, delete a user, or update a customer order, creating a dedicated task for each scenario provides a flexible and efficient solution. While tasks require manual execution, the investment is worthwhile for several reasons:

- **Automation of Manual Work:** Tasks automate manual processes, streamlining repetitive actions.
- **Utilization of Familiar Components:** Leverage your app's models, libraries, and existing logic within tasks.
- **Elimination of UI Development:** Tasks don't require building user interfaces, focusing solely on backend operations.
- **Potential for UI Automation:** If necessary, tasks can be automated with a UI by integrating with job-running tools like Jenkins.

Each task is designed to parse command-line arguments into flags, utilizing the yargs-parsed output of your CLI.

## Creating a Task with the CLI Generator

`Loco` provides a convenient code generator to simplify the creation of a starter task connected to your project. Use the following command to generate a task:

Generate the task:

<!-- <snip id="generate-task-help-command" inject_from="yaml" action="exec" template="sh"> -->
```sh
Generate a Task based on the given name

Usage: blo-cli generate task [OPTIONS] <NAME>

Arguments:
  <NAME>  Name of the thing to generate

Options:
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- </snip> -->

## Running a Task

Execute the task you created in the previous step using the following command:

<!-- <snip id="run-task-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco task <TASK_NAME>
```
<!-- </snip> -->


## Listing All Tasks

To view a list of all tasks that have been executed, use the following command:

<!-- <snip id="list-tasks-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco task
```
<!-- </snip> -->


## Creating a Task manually

If you prefer a manual approach to creating tasks in `Loco`, you can follow these steps:

#### 1. Create a Task File

Start by creating a new file under the path `src/tasks`. For example, let's create a file named `example.rs`:

<!-- <snip id="task-code-example" inject_from="code" template="rust"> -->
```rust
use loco_rs::prelude::*;

pub struct Foo;
#[async_trait]
impl Task for Foo {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "foo".to_string(),
            detail: "run foo task".to_string(),
        }
    }
    async fn run(&self, _app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        Ok(())
    }
}
```
<!-- </snip> -->

#### 2. Load the File in mod.rs

Next, ensure that you load the newly created task file in the `mod.rs` file within the `src/tasks` folder.

#### 3. Register the Task in App Hooks

In your App hook implementation (e.g., App struct), register the task in the register_tasks function:

```rust
// src/app.rs

pub struct App;
#[async_trait]
impl Hooks for App {
    ...

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::example::ExampleTask);
    }

    ...
}
```

These steps ensure that your manually created task, such as ExampleTask, is integrated into Loco's task management system.
