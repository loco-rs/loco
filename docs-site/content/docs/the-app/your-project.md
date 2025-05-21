+++
title = "Your Project"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2024-01-07T21:10:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

## Driving development with `cargo loco`

Create your starter app:

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Loco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

Now `cd` into your app and try out the various commands:

<!-- <snip id="help-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco --help
```
<!-- </snip> -->

<!-- <snip id="exec-help-command" inject_from="yaml" action="exec" template="sh"> -->
```sh
The one-person framework for Rust

Usage: demo_app-cli [OPTIONS] <COMMAND>

Commands:
  start       Start an app
  db          Perform DB operations
  routes      Describe all application endpoints
  middleware  Describe all application middlewares
  task        Run a custom task
  jobs        Managing jobs queue
  scheduler   Run the scheduler
  generate    code generation creates a set of files and code templates based on a predefined set of rules
  doctor      Validate and diagnose configurations
  version     Display the app version
  watch       Watch and restart the app
  help        Print this message or the help of the given subcommand(s)

Options:
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- </snip> -->


You can now drive your development through the CLI:

```
$ cargo loco generate model posts
$ cargo loco generate controller posts
$ cargo loco db migrate
$ cargo loco start
```

And running tests or working with Rust is just as you already know:

```
$ cargo build
$ cargo test
```

### Starting your app

To run you app, run:

<!-- <snip id="starting-the-server-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco start
```
<!-- </snip> -->

### Background workers

Based on your configuration (in `config/`), your workers will know how to operate:

```yaml
workers:
  # requires Redis
  mode: BackgroundQueue

  # can also use:
  # ForegroundBlocking - great for testing
  # BackgroundAsync - for same-process jobs, using tokio async
```

And now, you can run the actual process in various ways:

- `rr start --worker` - run only a worker and process background jobs. This is great for scale. Run one service app with `rr start`, and then run many process based workers with `rr start --worker` distributed on any machine you want.

* `rr start --server-and-worker` - will run both a service and a background worker processor in the same unix process. It uses Tokio for executing background jobs. This is great for those cases when you want to run on a single server without too much of an expense or have constrained resources.

### Getting your app version

Because your app is compiled, and then copied to production, Loco gives you two important operability pieces of information:

* Which version is this app, and which GIT SHA was it built from? `cargo loco version`
* Which Loco version was this app compiled against? `cargo loco --version`

Both version strings are parsable and stable so you can use it in integration scripts, monitoring tools and so on.

You can shape your own custom app versioning scheme by overriding the `app_version` hook in your `src/app.rs` file.


## Using the scaffold generator

Scaffolding is an efficient and speedy method for generating key components of an application. By utilizing scaffolding, you can create models, views, and controllers for a new resource all in one go.


See scaffold command:
<!-- <snip id="scaffold-help-command" inject_from="yaml" action="exec" template="sh"> -->
```sh
Generates a CRUD scaffold, model and controller

Usage: demo_app-cli generate scaffold [OPTIONS] <NAME> [FIELDS]...

Arguments:
  <NAME>       Name of the thing to generate
  [FIELDS]...  Model fields, eg. title:string hits:int

Options:
  -k, --kind <KIND>                The kind of scaffold to generate [possible values: api, html, htmx]
      --htmx                       Use HTMX scaffold
      --html                       Use HTML scaffold
      --api                        Use API scaffold
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- </snip> -->

You can begin by generating a scaffold for the Post resource, which will represent a single blog posting. To accomplish this, open your terminal and enter the following command:
<!-- <snip id="scaffold-post-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco generate scaffold posts name:string title:string content:text --api
```
<!-- </snip> -->

The scaffold generate command support API, HTML or HTMX by adding `--template` flag to scaffold command.

### Scaffold file layout

The scaffold generator will build several files in your application:

| File    | Purpose                                                                                                                                    |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| `migration/src/lib.rs`                     |  Include Post migration.                                                                                |
| `migration/src/m20240606_102031_posts.rs`  | Posts migration.                                                                                        |
| `src/app.rs`                               | Adding Posts to application router.                                                                     |
| `src/controllers/mod.rs`                   | Include the Posts controller.                                                                           |
| `src/controllers/posts.rs`                 | The Posts controller.                                                                                   |
| `tests/requests/posts.rs`                  | Functional testing.                                                                                     |
| `src/models/mod.rs`                        | Including Posts model.                                                                                  |
| `src/models/posts.rs`                      | Posts model,                                                                                            |
| `src/models/_entities/mod.rs`              | Includes Posts Sea-orm entity model.                                                                    |
| `src/models/_entities/posts.rs`            | Sea-orm entity model.                                                                                   |
| `src/views/mod.rs`                         | Including Posts views. only for HTML and HTMX templates.                                                |
| `src/views/posts.rs`                       | Posts template generator. only for HTML and HTMX templates.                                             |
| `assets/views/posts/create.html`           | Create post template. only for HTML and HTMX templates.                                                 |
| `assets/views/posts/edit.html`             | Edit post template. only for HTML and HTMX templates.                                                   |                                               |
| `assets/views/posts/list.html`             | List post template. only for HTML and HTMX templates.                                                   |
| `assets/views/posts/show.html`             | Show post template. only for HTML and HTMX templates.                                                   |

## Your app configuration
By default, loco stores its configuration files in the config/ directory. It provides predefined configurations for three environments:

```
config/
  development.yaml
  production.yaml
  test.yaml
```

An environment is picked up automatically based on:

- A command line flag: `cargo loco start --environment production`, if not given, fallback to
- `LOCO_ENV` or `RAILS_ENV` or `NODE_ENV`

When nothing is given, the default value is `development`.

The `Loco` framework allows support for custom environments in addition to the default environment. To add a custom environment, create a configuration file with a name matching the environment identifier used in the preceding example.

### Overriding the Default Configuration Path
To use a custom configuration directory, set the `LOCO_CONFIG_FOLDER` environment variable to the desired folder path. This will instruct `loco` to load configuration files from the specified directory instead of the default `config/` folder.

### Placeholders / variables in config

It is possible to inject values into a configuration file. In this example, we get a port value from the `NODE_PORT` environment variable:

```yaml
# config/development.yaml
# every configuration file is a valid Tera template
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port:  {{/* get_env(name="NODE_PORT", default=5150) */}}
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
  # Out of the box middleware configuration. to disable middleware you can changed the `enable` field to `false` of comment the middleware block
```

The [get_env](https://keats.github.io/tera/docs/#get-env) function is part of the Tera template engine. Refer to the [Tera](https://keats.github.io/tera/docs) docs to see what more you can use.

### Example

Suppose you want to add a 'qa' environment. Create a `qa.yaml` file in the config folder:

```
config/
  development.yaml
  production.yaml
  test.yaml
  qa.yaml
```

To run the application using the 'qa' environment, execute the following command:
<!-- <snip id="starting-the-server-command-with-environment-env-var" inject_from="yaml" template="sh"> -->
```sh
LOCO_ENV=qa cargo loco start
```
<!-- </snip> -->

### Settings

The configuration files contain knobs to set up your Loco app. You can also have your custom settings, with the `settings:` section. in `config/development.yaml` add the `settings:` section
<!-- <snip id="configuration-settings" inject_from="code" template="yaml"> -->
```yaml
settings:
  allow_list:
    - google.com
    - apple.com
```
<!-- </snip> -->

These setting will appear in `ctx.config.settings` as `serde_json::Value`. You can create your strongly typed settings by adding a struct:

```rust
// put this in src/common/settings.rs
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub allow_list: Option<Vec<String>>,
}

impl Settings {
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(value.clone())?)
    }
}
```

Then, you can access settings from anywhere like this:


```rust
// in controllers, workers, tasks, or elsewhere,
// as long as you have access to AppContext (here: `ctx`)

if let Some(settings) = &ctx.config.settings {
    let settings = common::settings::Settings::from_json(settings)?;
    println!("allow list: {:?}", settings.allow_list);
}
```

### Server



Here is a detailed description of the interface (listening, etc.) parameters under `server:`:

* `port:` as the name says, for changing ports, mostly when behind a load balancer, etc.

* `binding:` for changing what the IP interface "binds" to, mostly, when you are behind a load balancer like `nginx` you bind to a local address (when the LB is also there). However, you can also bind to "world" (`0.0.0.0`). You can set the binding: field via config, or via the CLI (using the `-b` flag) -- which is what Rails is doing.

* `host:` - for "visibility" use cases or out-of-band use cases. For example, sometimes you want to display the current server host (in terms of domain name, etc.), which serves for visibility. And sometimes, as in the case of emails -- your server address is "out of band", meaning when I open my gmail account and I have your email -- I have to click what looks like your external address or visible address (official domain name, etc), and not an internal "host" address which is what may be the wrong thing to do (imagine an email link pointing to "http://127.0.0.1/account/verify")



### Logger

Other than the commented fields in the `logger:` section on your YAML file, here's some more context:

* `logger.pretty_backtrace` - will display colorful backtrace without noise for great development experience. Note that this forcefully sets `RUST_BACKTRACE=1` into the process' env, which enables a (costly) backtrace capture on specific errors. Enable this in development, disable it in production. When needed in production, use `RUST_BACKTRACE=1` ad-hoc in the command line to show it.


For all available configuration options [click here](https://docs.rs/loco-rs/latest/loco_rs/config/struct.Config.html)
