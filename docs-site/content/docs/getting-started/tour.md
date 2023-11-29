+++
title = "A Tour with Loco"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

Let's create a blog on `loco` in 3 commands. First install `loco-cli`:


```sh
$ cargo install loco-cli
```

Now you can create your new app (choose "Saas app").

```sh
$ loco new
❯ App name? [myapp]:
❯ Saas app (with DB and user auth)
  Stateless service (minimal, no db)
🚂 Loco app generated successfully in:
myapp
```

> To configure a database , please run a local postgres database with `loco:loco` and a db named `loco_app`: 
`docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=loco_app -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine`

Now `cd` into your `myapp` and start your app:

```
$ cd myapp
$ cargo loco run
Finished dev [unoptimized + debuginfo] target(s) in 21.63s
    Running `target/debug/myapp start`

    :
    :
    :

controller/app_routes.rs:203: [Middleware] Adding log trace id

                      ▄     ▀
                                 ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
 ██████  █████   ███ █████   ███ █████   ███ ▀█
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀

started on port 3000
```

## Adding a CRUD API

We have a base SaaS app with user authentication generated for us. Let's make it a blog by adding a `post` and a full CRUD API using `scaffold`:

```sh
$ cargo loco generate scaffold post title:string content:text

  :
  :
added: "src/controllers/post.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/post.rs"
injected: "tests/requests/mod.rs"
* Migration for `post` added! You can now apply it with `$ lc db migrate`.
* A test for model `posts` was added. Run with `cargo test`.
* Controller `post` was added successfully.
* Tests for controller `post` was added successfully. Run `cargo test`.
```

Your database have been migrated and model, entities, and a full CRUD controller have been generated automatically.

Start your app:

```sh
$ cargo loco start
```

Next, try adding a `post` with `curl`:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:3000/posts
```
You can list your posts:

```sh
$ curl localhost:3000/posts
```

For those counting -- the 3 commands for creating a blog were:

1. `cargo install loco-cli`
2. `loco new`
3. `cargo loco generate scaffold post title:string content:text`

Done! enjoy your ride with `loco` 🚂

## Checking Out SaaS Authentication

Your generated app contains a fully working authentication suite, based on JWTs.

