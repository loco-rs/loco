+++
title = "Controller"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 7
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

`Loco` is a framework that wraps around [axum](https://crates.io/crates/axum), offering a straightforward approach to manage routes, middlewares, authentication, and more right out of the box. At any point, you can leverage the powerful axum Router and extend it with your custom middlewares and routes.

### Router Capabilities

`Loco` router provides several capabilities:

#### Defining App Routes in the App Hook

In the example below, multiple controllers are added to your app within the `AppRouter`. During initialization, you can choose between:

- **AppRoutes::with_default_routes():** Adds default loco endpoints like ping or heathy.
- **AppRoutes::empty():** Creates an empty router without default routes.

```rust
pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::foo::routes())
            .add_route(controllers::bar::routes())
    }
    ...
}
```

#### Adding a Prefix to All Routes

You can add a prefix URL to all your routes by providing the prefix to the AppRouter instance.

### Routes in Controllers

Controllers define Loco routes capabilities. In the example below, a controller creates one GET endpoint and one POST endpoint:

```rust
use axum::routing::{get, post};
Routes::new()
    .add("/", get(hello))
    .add("/echo", post(echo))
```

You can also define a `prefix` for all routes in a controller using the `prefix` function.

## Creating a Controller with the CLI Generator

Provides a convenient code generator to simplify the creation of a starter controller connected to your project. Additionally, a [test](@/docs/testing/controller.md) file is generated, enabling easy testing of your controller.

Generate a controller:

```sh
$ rr generate controller [OPTIONS] <CONTROLLER_NAME>
```

After generating the controller, navigate to the created file in `src/controllers` to view the controller endpoints. You can also check the testing (in folder tests/requests) documentation for testing this controller.

## Creating a Controller Manually

#### 1. Create a Controller File

Start by creating a new file under the path `src/controllers`. For example, let's create a file named `example.rs`.

#### 2. Load the File in mod.rs

Ensure that you load the newly created controller file in the `mod.rs` file within the `src/controllers` folder.

#### 3. Register the Controller in App Hooks

In your App hook implementation (e.g., App struct), add your controller's `Routes` to `AppRoutes`:

```rust
// src/app.rs

pub struct App;
#[async_trait]
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes().prefix("prefix")
            .add_route(controllers::example::routes())
    }
    ...
}

```
