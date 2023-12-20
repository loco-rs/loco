[![Current Crates.io Version](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
[![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/Npcwuvq6)

# Welcome to Loco!

<center>
<img width="640" src="https://github.com/loco-rs/loco/raw/master/media/image.png"/>
</center>
<br/>
<center>
<a href="https://loco.rs">loco.rs</a>
</center>
<br/>
<br/>

Loco is "Rust on Rails".

Loco is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

To work with Loco, you need to know Rust to a beginner-moderate level. There are no crazy lifetime twisters and most of the development will be linear: request handling, workers, tasks, etc.

## Quick Start

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
To configure a database , please run a local postgres database with <code>loco:loco</code> and a db named <code>loco_app</code>.
</div>

You can use Docker to run a Postgres instance:

```
$ docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=loco_app -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine
```

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

## Project Status

Loco is feature complete, but features are still being added rapidly.

### What can you build?

- Stateless APIs
- Complete SaaS products with user authentication
- Purpose-built services such as ML inference endpoints
- Full stack projects with separate frontend project integrated with Loco
- Hobby projects full-stack with backend and HTML frontend

### What's being done now?

- View [issues](https://github.com/loco-rs/loco/issues) for what we plan next and what we work on (you're welcome to submit PRs!)
- View [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md) for what we already added
