+++
title = "A Quick Tour with Loco"
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

<img style="width:100%; max-width:640px" src="tour.png"/>
<br/>
<br/>
<br/>
Let's create a blog on `loco` in 4 commands. First install `loco-cli` and `sea-orm-cli`:

```sh
$ cargo install loco-cli
$ cargo install sea-orm-cli
```

Now you can create your new app (choose "React Frontend").

```sh
$ loco new
❯ App name? [myapp]:
❯ React Frontend (with DB and user auth)
  Stateless service (minimal, no db)
🚂 Loco app generated successfully in:
myapp
```

<div class="infobox">
To configure a database , please run a local postgres database with <code>loco:loco</code> and a db named is the [insert app]_development.
</div>

You can use Docker to run a Postgres instance:

When generating a starter, the database name incorporates your application name and the environment. For instance, if you include `myapp`, the database name in the `test.yaml`configuration will be `myapp_test`, and in the `development.yaml` configuration, it will be `myapp_development`.

```
$ docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=myapp_development -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine
```

A more advanced set of `docker-compose.yml` and `Dockerfiles` that include Redis and the `mailtutan` mailer are available for [each starter on GitHub](https://github.com/loco-rs/loco/blob/master/starters/react/.devcontainer/docker-compose.yml).

Now `cd` into your `myapp` and start your app:

```
$ cd myapp
$ cargo loco start
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

<div class="infobox">
You don't have to run things through `cargo` but in development it's highly recommended. If you build `--release`, your binary contains everything including your code and `cargo` or Rust is not needed.
</div>

## Adding a CRUD API

We have a base React Frontend with user authentication generated for us. Let's make it a blog by adding a `post` and a full CRUD API using `scaffold`:

```sh
$ cargo loco generate scaffold post title:string content:text

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

Start your app:

```sh
$ cargo loco start
```

Next, try adding a `post` with `curl`:

```sh
$ curl -X POST -H "Content-Type: application/json" -d '{
  "title": "Your Title",
  "content": "Your Content xxx"
}' localhost:3000/api/posts
```

You can list your posts:

```sh
$ curl localhost:3000/api/posts
```

For those counting -- the commands for creating a blog were:

1. `cargo install loco-cli`
2. `cargo install sea-orm-cli`
3. `loco new`
4. `cargo loco generate scaffold post title:string content:text`

Done! enjoy your ride with `loco` 🚂

## Checking Out SaaS/React Authentication

Your generated app contains a fully working authentication suite, based on JWTs.

### Registering a New User

The `/api/auth/register` endpoint creates a new user in the database with an `email_verification_token` for account verification. A welcome email is sent to the user with a verification link.

```sh
$ curl --location '127.0.0.1:3000/api/auth/register' \
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
$ curl --location '127.0.0.1:3000/api/auth/login' \
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
    "is_verified": false
}
```

### Get current user

This endpoint is protected by auth middleware.

```sh
$ curl --location --request GET '127.0.0.1:3000/api/user/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN'
```

Check out the source code for `controllers/auth.rs` to see how to use the authentication middleware in your own controllers.
