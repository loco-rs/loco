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

Loco is a Rust API and web framework for full stack product builders.

You need to be familiar with Rust to a moderate level. You need to know how to build, test, and run Rust projects, have used some popular libraries such as clap, regex, tokio, axum or other web framework, nothing too fancy. There are no crazy lifetime twisters or complex / too magical, macros in Loco that you need to know how they work.

Loco is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

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

- [x] `STAGE 1`: **wide but shallow**. Build a lot of components that make up a Rails-like framework, but only invest 60% in what they should be.
- [x] `STAGE 2`: **operable demo**. Build tooling - CLI, testing, and a demo app to showcase the framework
- [x] `STAGE 3`: **tooling polish**. Make day to day _development_ operations easy, such as adding models, controllers, logic and tests (by code generation, macros, or helpers)
- [ ] `STAGE 4`: **indepth development**. Make the 60% go to 80% of functionality where needed. Things like documentation (website?), missing API, supporting various deployment scenarios (on docker, other platforms), feature flags to cut down functionality
- [ ] `STAGE 5`: **go wide and expand**. Focus on building various kinds of demo apps
