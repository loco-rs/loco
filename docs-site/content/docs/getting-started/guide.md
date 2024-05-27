+++
title = "The Loco Guide"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

## Guide Assumptions

Loco is a Rust API and web framework for full stack product builders.

The name `Loco` comes from **loco**motive, as a tribute to Rails, and `loco` is easier to type than `locomotive` :-). Also, in some languages it means "crazy" but that was not the original intention (or, is it crazy to build a Rails on Rust? only time will tell!).

You need to be familiar with Rust to a moderate level. You need to know how to build, test, and run Rust projects, have used some popular libraries such as `clap`, `regex`, `tokio`, `axum` or other web framework, nothing too fancy. There are no crazy lifetime twisters or complex / too magical, macros in Loco that you need to know how they work.

Loco is strongly inspired by Rails. If you know Rails _and_ Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

<div class="infobox">
We think Rails is so great, that this guide is strongly inspired from the <a href="https://guides.rubyonrails.org/getting_started.html">Rails guide, too</a>
</div>

## What is Loco?

Loco is a Web or API framework for Rust. It's also a productivity suite for developers: it contains everything you need while building a hobby or your next startup. It's also strongly inspired by Rails.

- **You have a variant of the MVC model**, which removes the paradox of option. You deal with building your app, not making academic decisions for what abstractions to use.
- **Fat models, slim controllers**. Models should contain most of your logic and business implementation, controllers should just be a lightweight router that understands HTTP and moves parameters around.
- **Command line driven** to keep your momentum and flow. Generate stuff over copying and pasting or coding from scratch.
- **Every task is "infrastructure-ready"**, just plug in your code and wire it in: controllers, models, views, tasks, background jobs, mailers, and more.
- **Convention over configuration**: decisions are already done for you -- the folder structure matter, configuration shape and values matter, and the way an app is wired matter to how an app operates and for you do be the most effective.

## Creating a New Loco App

You can follow this guide for a step-by-step "bottom up" learning, or you can jump and go with the [tour](./tour.md) instead for a quicker "top down" intro.

### Installing

<!-- <snip id="quick-installation-command" inject_from="yaml"> -->
```sh
cargo install loco-cli
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->


### Creating a new Loco app

Now you can create your new app (choose "SaaS app" for built-in authentication).

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)

🚂 Loco app generated successfully in:
myapp
```
<!-- </snip> -->


You can now switch to to `myapp`:

```sh
$ cd myapp
```

Make sure you also have locally installed or running (via Docker or otherwise) in case you selected starter with DB dependencies:

- Postgres (your database will be named `myapp_development`)
- Redis

<div class="infobox">
To configure a database, please run a local postgres database with <code>loco:loco</code> and a db named <code>myapp_development</code>.
</div>

This docker command start up postgresql database server.

<!-- <snip id="postgres-run-docker-command" inject_from="yaml"> -->
```sh
docker run -d -p 5432:5432 \
  -e POSTGRES_USER=loco \
  -e POSTGRES_DB=myapp_development \
  -e POSTGRES_PASSWORD="loco" \
  postgres:15.3-alpine
```
<!-- </snip> -->

This docker command start up redis server:

```
docker run -p 6379:6379 -d redis redis-server
```

Use doctor command to check the needed resources:

<!-- <snip id="doctor-command" inject_from="yaml"> -->
```sh
$ cargo loco doctor
    Finished dev [unoptimized + debuginfo] target(s) in 0.32s
    Running `target/debug/myapp-cli doctor`
✅ SeaORM CLI is installed
✅ DB connection: success
✅ Redis connection: success
```
<!-- </snip> -->

Here's a rundown of what Loco creates for you by default:

| File/Folder    | Purpose                                                                                                                                                           |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/`         | Contains controllers, models, views, tasks and more                                                                                                               |
| `app.rs`       | Main component registration point. Wire the important bits here.                                                                                                  |
| `lib.rs`       | Various rust-specific exports of your components.                                                                                                                 |
| `bin/`         | Has your `main.rs` file, you don't need to worry about it                                                                                                         |
| `controllers/` | Contains controllers, all controllers are exported via `mod.rs`                                                                                                   |
| `models/`      | Contains models, `models/_entities` contains auto-generated SeaORM models, and `models/*.rs` contains your model extension logic, which are exported via `mod.rs` |
| `views/`       | Contains JSON-based views. Structs which can `serde` and output as JSON through the API.                                                                          |
| `workers/`     | Has your background workers.                                                                                                                                      |
| `mailers/`     | Mailer logic and templates, for sending emails.                                                                                                                   |
| `fixtures/`    | Contains data and automatic fixture loading logic.                                                                                                                |
| `tasks/`       | Contains your day to day business-oriented tasks such as sending emails, producing business reports, db maintenance, etc.                                         |
| `tests/`       | Your app-wide tests: models, requests, etc.                                                                                                                       |
| `config/`      | A stage-based configuration folder: development, test, production                                                                                                 |
| `channels/`    | Contains all channels routes.                                                                                                                                     |

## Hello, Loco!

Let's get some responses quickly. For this, we need to start up the server.

### Starting the server

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

And now, let's see that it's alive:

```sh
$ curl localhost:3000/_ping
{"ok":true}
```

The built in `_ping` route will tell your load balancer everything is up.

Let's see that all services that are required are up:

```sh
$ curl localhost:3000/_health
{"ok":true}
```

<div class="infobox">
The built in <code>_health</code> route will tell you that you have configured your app properly: it can establish a connection to your Postgres and Redis instances successfully.
</div>

### Say "Hello", Loco

Let's add a quick _hello_ response to our service.

```sh
$ cargo loco generate controller guide
added: "src/controllers/guide.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/guide.rs"
injected: "tests/requests/mod.rs"
```

This is the generated controller body:

```rust
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

pub async fn echo(req_body: String) -> String {
    req_body
}

pub async fn hello(State(_ctx): State<AppContext>) -> Result<Response> {
    // do something with context (database, etc)
    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("guide")
        .add("/", get(hello))
        .add("/echo", post(echo))
}
```

Start the server:

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

Now, let's test it out:

```sh
$ curl localhost:3000/guide
hello
```

Loco has powerful generators, which will make you 10x productive and drive your momentum when building apps.

If you'd like to be entertained for a moment, let's "learn the hard way" and add a new controller manually as well.

Add a file called `home.rs`, and `pub mod home;` it in `mod.rs`:

```
src/
  controllers/
    auth.rs
    home.rs      <--- add this file
    users.rs
    mod.rs       <--- 'pub mod home;' the module here
```

Next, set up a _hello_ route, this is the contents of `home.rs`:

```rust
// src/controllers/home.rs
use loco_rs::prelude::*;

// _ctx contains your database connection, as well as other app resource that you'll need
async fn hello(State(_ctx): State<AppContext>) -> Result<Response> {
    format::text("ola, mundo")
}

pub fn routes() -> Routes {
    Routes::new().prefix("home").add("/hello", get(hello))
}
```

Finally, register this new controller routes in `app.rs`:

```rust
src/
  controllers/
  models/
  ..
  app.rs   <---- look here
```

Add the following in `routes()`:

```rust
// in src/app.rs
#[async_trait]
impl Hooks for App {
    ..
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::guide::routes())
            .add_route(controllers::notes::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::user::routes())
            .add_route(controllers::home::routes()) // <--- add this
    }
```

That's it. Kill the server and bring it up again:

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

And hit `/home/hello`:

```sh
$ curl localhost:3000/home/hello
ola, mundo
```

You can take a look at all of your routes with:

```
$ cargo loco routes
  ..
  ..
[POST] /api/auth/login
[POST] /api/auth/register
[POST] /api/auth/reset
[POST] /api/auth/verify
[GET] /home/hello      <---- this is our new route!
[GET] /api/notes
[POST] /api/notes
  ..
  ..
$
```

<div class="infobox">
The <em>SaaS Starter</em> keeps routes under <code>/api</code> because it is client-side ready. <br/>
When using client-side routing like React Router, we want to separate backend routes from client routes: the browser will use <code>/home</code> but not <code>/api/home</code> which is the backend route, and you can call <code>/api/home</code> from the client with no worries. Nevertheless, the routes: <code>/_health</code> and <code>/_ping</code> are exceptions, they stay at the root.
</div>

## MVC and You

**Traditional MVC (Model-View-Controller) originated in desktop UI programming paradigms.** However, its applicability to web services led to its rapid adoption. MVC's golden era was around the early 2010s, and since then, many other paradigms and architectures have emerged.

**MVC is still a very strong principle and architecture to follow for simplifying projects**, and this is what Loco follows too.

Although web services and APIs don't have a concept of a _view_ because they do not generate HTML or UI responses, **we claim _stable_, _safe_ services and APIs indeed has a notion of a view** -- and that is the serialized data, its shape, its compatibility and its version.

```
// a typical loco app contains all parts of MVC

src/
  controllers/
    users.rs
    mod.rs
  models/
    _entities/
      users.rs
      mod.rs
    users.rs
    mod.rs
  views/
    users.rs
    mod.rs
```

**This is an important _cognitive_ principle**. And the principle claims that you can only create safe, compatible API responses if you treat those as a separate, independently goverened _thing_ -- hence the 'V' in MVC, in Loco.

<div class="infobox">
Models in Loco carry the same semantics as in Rails: <b>fat models, slim controllers</b>. This means that every time you want to build something -- <em>you reach out to a model</em>.
</div>

### Generating a model

A model in Loco represents data. Typically that data is stored in your database. Most, if not all, business processes of your applications would be coded on the model (as an Active Record) or as an orchestration of a few models.

Let's create a new model called `Article`:

```sh
$ cargo loco generate model article title:string content:text

added: "migration/src/m20231202_173012_articles.rs"
injected: "migration/src/lib.rs"
injected: "migration/src/lib.rs"
added: "tests/models/articles.rs"
injected: "tests/models/mod.rs"
```

### Database migrations

**Keeping your schema is done with migrations**. A migration is a singular change to your database structure: it can contain complete table additions, modifications, or index creation.

```rust
// this was generated into `migrations/` from the command:
//
// $ cargo loco generate model article title:string content:text
//
// it is automatically applied by Loco's migrator framework.
// you can also apply it manually using the command:
//
// $ cargo loco db migrate
//
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                table_auto(Articles::Table)
                    .col(pk_auto(Articles::Id))
                    .col(string_null(Articles::Title))
                    .col(text(Articles::Content))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Articles::Table).to_owned())
            .await
    }
}
```

You can recreate a complete database **by applying migrations in-series onto a fresh database schema** -- this is done automatically by Loco's migrator (which is derived from SeaORM).

When generating a new model, Loco will:

- Generate a new "up" database migration
- Apply the migration
- Reflect the entities from database structure and generate back your `_entities` code

You will find your new model as an entity, synchronized from your database structure in `models/_entities/`:

```
src/models/
├── _entities
│   ├── articles.rs  <-- sync'd from db schema, do not edit
│   ├── mod.rs
│   ├── notes.rs
│   ├── prelude.rs
│   └── users.rs
├── articles.rs   <-- generated for you, your logic goes here.
├── mod.rs
├── notes.rs
└── users.rs
```

### Using `playground` to interact with the database

Your `examples/` folder contains:

- `playground.rs` - a place to try out and experiment with your models and app logic.

Let's fetch data using your models, using `playground.rs`:

```rust
// located in examples/playground.rs
// use this file to experiment with stuff
use eyre::Context;
use loco_rs::{cli::playground, prelude::*};
// to refer to articles::ActiveModel, your imports should look like this:
use myapp::{app::App, models::_entities::articles};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let ctx = playground::<App>().await.context("playground")?; // <- remove '_'

    // add this:
    let res = articles::Entity::find().all(&ctx.db).await.unwrap();
    println!("{:?}", res);

    Ok(())
}

```

### Return a list of posts

In the example, we use the following to return a list:

```rust
let res = articles::Entity::find().all(&ctx.db).await.unwrap();
```

To see how to run more queries, go to the [SeaORM docs](https://www.sea-ql.org/SeaORM/docs/next/basic-crud/select/).

To execute your playground, run:

```rust
$ cargo playground
[]
```

Now, let's insert one item:

```rust
async fn main() -> eyre::Result<()> {
    let ctx = playground::<App>().await.context("playground")?;

    // add this:
    let active_model: articles::ActiveModel = articles::ActiveModel {
        title: Set(Some("how to build apps in 3 steps".to_string())),
        content: Set(Some("use Loco: https://loco.rs".to_string())),
        ..Default::default()
    };
    active_model.insert(&ctx.db).await.unwrap();

    let res = articles::Entity::find().all(&ctx.db).await.unwrap();
    println!("{:?}", res);

    Ok(())
}
```

And run the playground again:

```sh
$ cargo playground
[Model { created_at: ..., updated_at: ..., id: 1, title: Some("how to build apps in 3 steps"), content: Some("use Loco: https://loco.rs") }]
```

We're now ready to plug this into an `articles` controller. First, generate a new controller:

```sh
$ cargo loco generate controller articles
added: "src/controllers/articles.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/articles.rs"
injected: "tests/requests/mod.rs"
```

Edit `src/controllers/articles.rs`:

```rust
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::models::_entities::articles;

pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let res = articles::Entity::find().all(&ctx.db).await?;
    format::json(res)
}

pub fn routes() -> Routes {
    Routes::new().prefix("articles").add("/", get(list))
}
```

Now, start the app:

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

And make a request:

```sh
$ curl localhost:3000/articles
[{"created_at":"...","updated_at":"...","id":1,"title":"how to build apps in 3 steps","content":"use Loco: https://loco.rs"}]
```

## Building a CRUD API

Next we'll see how to get a single article, delete, and edit a single article. Getting an article by ID is done using the `Path` extractor from `axum`.

Replace the contents of `articles.rs` with this:

```rust
// this is src/controllers/articles.rs

#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::_entities::articles::{ActiveModel, Entity, Model};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub title: Option<String>,
    pub content: Option<String>,
}

impl Params {
    fn update(&self, item: &mut ActiveModel) {
        item.title = Set(self.title.clone());
        item.content = Set(self.content.clone());
    }
}

async fn load_item(ctx: &AppContext, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.db).await?;
    item.ok_or_else(|| Error::NotFound)
}

pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Entity::find().all(&ctx.db).await?)
}

pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let mut item = ActiveModel {
        ..Default::default()
    };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}

pub async fn update(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    let mut item = item.into_active_model();
    params.update(&mut item);
    let item = item.update(&ctx.db).await?;
    format::json(item)
}

pub async fn remove(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<()> {
    load_item(&ctx, id).await?.delete(&ctx.db).await?;
    format::empty()
}

pub async fn get_one(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    format::json(load_item(&ctx, id).await?)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("articles")
        .add("/", get(list))
        .add("/", post(add))
        .add("/:id", get(get_one))
        .add("/:id", delete(remove))
        .add("/:id", post(update))
}
```

A few items to note:

- `Params` is a strongly typed required params data holder, and is similar in concept to Rails' _strongparams_, just safer.
- `Path(id): Path<i32>` extracts the `:id` component from a URL.
- Order of extractors is important and follows `axum`'s documentation (parameters, state, body).
- It's always better to create a `load_item` helper function and use it in all singular-item routes.
- While `use loco_rs::prelude::*` brings in anything you need to build a controller, you should note to import `crate::models::_entities::articles::{ActiveModel, Entity, Model}` as well as `Serialize, Deserialize` for params.


<div class="infobox">
The order of the extractors is important, as changing the order of them can lead to compilation errors. Adding the <code>#[debug_handler]</code> macro to handlers can help by printing out better error messages. More information about extractors can be found in the <a href="https://docs.rs/axum/latest/axum/extract/index.html#the-order-of-extractors">axum documentation</a>.
</div>


You can now test that it works, start the app:

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

Add a new article:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:3000/articles
{"created_at":"...","updated_at":"...","id":2,"title":"Your Title","content":"Your Content xxx"}
```

Get a list:

```sh
$ curl localhost:3000/articles
[{"created_at":"...","updated_at":"...","id":1,"title":"how to build apps in 3 steps","content":"use Loco: https://loco.rs"},{"created_at":"...","updated_at":"...","id":2,"title":"Your Title","content":"Your Content xxx"}
```

## Adding a second model

Let's add another model, this time: `Comment`. We want to create a relation - a comment belongs to a post, and each post can have multiple comments.

Instead of coding the model and controller by hand, we're going to create a **comment scaffold** which will generate a fully working CRUD API comments. We're also going to use the special `references` type:

```sh
$ cargo loco generate scaffold comment content:text article:references
```

If you peek into the new migration, you'll discover a new database relation in the articles table:

```rust
      ..
      ..
  .col(integer(Comments::ArticleId))
  .foreign_key(
      ForeignKey::create()
          .name("fk-comments-articles")
          .from(Comments::Table, Comments::ArticleId)
          .to(Articles::Table, Articles::Id)
          .on_delete(ForeignKeyAction::Cascade)
          .on_update(ForeignKeyAction::Cascade),
  )
      ..
      ..
```

Now, lets modify our API in the following way:

1. Comments can be added through a shallow route: `POST comments/`
2. Comments can only be fetched in a nested route (forces a Post to exist): `GET posts/1/comments`
3. Comments cannot be updated, fetched singular, or deleted

In `src/controllers/comments.rs`, remove unneeded routes and functions:

```rust
pub fn routes() -> Routes {
    Routes::new()
        .prefix("comments")
        .add("/", post(add))
        // .add("/", get(list))
        // .add("/:id", get(get_one))
        // .add("/:id", delete(remove))
        // .add("/:id", post(update))
}
```

Also adjust the Params & update functions in `src/controllers/comments.rs`, by updating the scaffolded code marked with `<- add this`

```rust
pub struct Params {
    pub content: Option<String>,
    pub article_id: i32, // <- add this
}

impl Params {
    fn update(&self, item: &mut ActiveModel) {
        item.content = Set(self.content.clone());
        item.article_id = Set(self.article_id.clone()); // <- add this
    }
}
```

Now we need to fetch a relation in `src/controllers/articles.rs`. Add the following route:

```rust
pub fn routes() -> Routes {
  // ..
  // ..
  .add("/:id/comments", get(comments))
}
```

And implement the relation fetching:

```rust
// to refer to comments::Entity, your imports should look like this:
use crate::models::_entities::{
    articles::{ActiveModel, Entity, Model},
    comments,
};

pub async fn comments(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    let comments = item.find_related(comments::Entity).all(&ctx.db).await?;
    format::json(comments)
}
```

<div class="infobox">
This is called "lazy loading", where we fetch the item first and later its associated relation. Don't worry - there is also a way to eagerly load comments along with an article.
</div>

Now let's add a comment to Article `1`:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "content": "this rocks",
  "article_id": 1
}' localhost:3000/comments
{"created_at":"...","updated_at":"...","id":4,"content":"this rocks","article_id":1}
```

And, fetch the relation:

```sh
$ curl localhost:3000/articles/1/comments
[{"created_at":"...","updated_at":"...","id":4,"content":"this rocks","article_id":1}]
```

This ends our comprehensive _Guide to Loco_. If you made it this far, hurray!.

## Tasks: export data report

Real world apps require handling real world situations. Say some of your users or customers require some kind of a report.

You can:

- Connect to your production database, issue ad-hoc SQL queries. Or use some kind of DB tool. _This is unsafe, insecure, prone to errors, and cannot be automated_.
- Export your data to something like Redshift, or Google, and issue a query there. _This is a waste of resource, insecure, cannot be tested properly, and slow_.
- Build an admin. _This is time-consuming, and waste_.
- **Or build an adhoc task in Rust, which is quick to write, type safe, guarded by the compiler, fast, environment-aware, testable, and secure.**

This is where `cargo loco task` comes in.

First, run `cargo loco task`:

```sh
$ cargo loco task
user_report		[output a user report]
```

You'll see an example task that was generated for you. This is the meat of the task:

```rust
// find it in `src/tasks/user_report.rs`
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
      // description that appears on the CLI
        TaskInfo {
            name: "user_report".to_string(),
            detail: "output a user report".to_string(),
        }
    }

    // variables through the CLI:
    // `$ cargo loco task name:foobar count:2`
    // will appear as {"name":"foobar", "count":2} in `vars`
    async fn run(&self, app_context: &AppContext, vars: &BTreeMap<String, String>) -> Result<()> {
        let users = users::Entity::find().all(&app_context.db).await?;
        println!("args: {vars:?}");
        println!("!!! user_report: listing users !!!");
        println!("------------------------");
        for user in &users {
            println!("user: {}", user.email);
        }
        println!("done: {} users", users.len());
        Ok(())
    }
}
```

You can modify this task as you see fit. Access the models with `app_context`, or any other environmental resources, and fetch
variables that were given through the CLI with `vars`.

Running this task is done with:

```rust
$ cargo loco task user_report var1:val1 var2:val2 ...
```

Remember: this is environmental, so you write the task once, and then execute in development or production as you wish. Tasks are compiled into the main app binary.

## Authentication: authenticating your requests

If you chose the `SaaS App` starter, you should have a fully configured authentication module baked into the app.
Let's see how to require authentication when **adding comments**.

Go back to `src/controllers/comments.rs` and take a look at the `add` function:

```rust
pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let mut item = ActiveModel {
        ..Default::default()
    };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}
```

To require authentication, we need to modify the function signature in this way:

```rust
async fn add(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
  // we only want to make sure it exists
  let _current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

  // next, update
  // homework/bonus: make a comment _actually_ belong to user (user_id)
  let mut item = ActiveModel {
      ..Default::default()
  };
  params.update(&mut item);
  let item = item.insert(&ctx.db).await?;
  format::json(item)
}
```
