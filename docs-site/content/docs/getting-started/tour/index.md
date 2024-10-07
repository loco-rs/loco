+++
title = "A Quick Tour"
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

English - [Français](./index.fr.md)

<img style="width:100%; max-width:640px" src="tour.png"/>
<br/>
<br/>
<br/>
Let's create a blog backend on Loco in just a few minutes. First install `loco-cli` and `sea-orm-cli`:

<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco-cli
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->


 Now you can create your new app (choose "`SaaS` app").

 ```sh
 ❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokyo async tasks)
✔ ❯ Select an asset serving configuration · Client (configures assets for frontend serving)

 🚂 Loco app generated successfully in:
 myapp/
 ```

If you select all defaults, you'll have:

* `sqlite` for database. Learn about database providers in [Sqlite vs Postgres](@/docs/the-app/models.md#sqlite-vs-postgres) in the _models_ section.
* `async` for background workers. Learn about workers configuration [async vs queue](@/docs/processing/workers.md#async-vs-queue) in the _workers_ section.
* `Client` asset serving configuration. This means your backend will serve as API.


 Now `cd` into your `myapp` and start your app by running `cargo loco start`:

<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo loco start

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
                https://loco.rs

listening on port 5150
```
<!-- </snip> -->


 <div class="infobox">
 You don't have to run things through `cargo` but in development it's highly
 recommended. If you build `--release`, your binary contains everything
 including your code and `cargo` or Rust is not needed. </div>

## Adding a CRUD API

We have a base SaaS app with user authentication generated for us. Let's make it a blog backend by adding a `post` and a full CRUD API using `scaffold`:

```sh
$ cargo loco generate scaffold post title:string content:text -k api

  :
  :
added: "src/controllers/post.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/post.rs"
injected: "tests/requests/mod.rs"
* Migration for `post` added! You can now apply it with `$ cargo loco db migrate`.
* A test for model `posts` was added. Run with `cargo test`.
* Controller `post` was added successfully.
* Tests for controller `post` was added successfully. Run `cargo test`.
```

Your database have been migrated and model, entities, and a full CRUD controller have been generated automatically.

Start your app again:
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo loco start

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
                https://loco.rs

listening on port 5150
```
<!-- </snip> -->

Next, try adding a `post` with `curl`:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:5150/posts
```

You can list your posts:

```sh
$ curl localhost:5150/posts
```

For those counting -- the commands for creating a blog backend were:

1. `cargo install loco-cli`
2. `cargo install sea-orm-cli`
3. `loco new`
4. `cargo loco generate scaffold post title:string content:text -k api`

Done! enjoy your ride with `loco` 🚂

## Checking Out SaaS Authentication

Your generated app contains a fully working authentication suite, based on JWTs.

### Registering a New User

The `/api/auth/register` endpoint creates a new user in the database with an `email_verification_token` for account verification. A welcome email is sent to the user with a verification link.

```sh
$ curl --location '127.0.0.1:5150/api/auth/register' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "name": "Loco user",
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

For security reasons, if the user is already registered, no new user is created, and a 200 status is returned without exposing user email details.

### Login

After registering a new user, use the following request to log in:

```sh
$ curl --location '127.0.0.1:5150/api/auth/login' \
     --header 'Content-Type: application/json' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "12341234"
     }'
```

The response includes a JWT token for authentication, user ID, name, and verification status.

```sh
{
    "token": "...",
    "pid": "2b20f998-b11e-4aeb-96d7-beca7671abda",
    "name": "Loco user",
    "claims": null
    "is_verified": false
}
```

In your client-side app, you save this JWT token and make following requests with it using _bearer token_ (see below) in order for those to be authenticated.

### Get current user

This endpoint is protected by auth middleware. We will use the token we got earlier to perform a request with the _bearer token_ technique (replace `TOKEN` with the JWT token you got earlier):

```sh
$ curl --location --request GET '127.0.0.1:5150/api/user/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN'
```

That should be your first authenticated request!.

Check out the source code for `controllers/auth.rs` to see how to use the authentication middleware in your own controllers.
