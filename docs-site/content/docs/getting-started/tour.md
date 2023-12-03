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

<div class="infobox">
To configure a database , please run a local postgres database with <code>loco:loco</code> and a db named <code>loco_app</code>: 
<code>docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=loco_app -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine</code>
</div>

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

### Registering a New User

The `/auth/register` endpoint creates a new user in the database with an `email_verification_token` for account verification. A welcome email is sent to the user with a verification link.


```sh
$ curl --location '127.0.0.1:3000/auth/register' \
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
$ curl --location '127.0.0.1:3000/auth/login' \
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
$ curl --location --request GET '127.0.0.1:3000/user/current' \
     --header 'Content-Type: application/json' \
     --header 'Authorization: Bearer TOKEN' \
     --data-raw '{
         "email": "user@loco.rs",
         "password": "new-password"
     }'
```


Check out the source code for `controllers/auth.rs` to see how to use the authentication middleware in your own controllers.
