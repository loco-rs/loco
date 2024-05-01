
![Loco.rs](https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac)
[![Current Crates.io Version](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
[![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

# Welcome to Loco!

<a href="https://loco.rs">https://loco.rs</a>


Loco is "Rust on Rails".

Loco is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.


## Quick Start

```sh
$ cargo install loco-cli
```

Now you can create your new app (choose "SaaS app").

```sh
$ loco new
❯ App name? [myapp]:
❯ SaaS app (with DB and user auth)
  Stateless service (minimal, no db)
🚂 Loco app generated successfully in:
myapp
```

To configure a database , please run a local postgres database with <code>loco:loco</code> and a db named <code>[insert app]_development</code>.

```
$ docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=myapp_development -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine
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

## Powered by Loco

* [SpectralOps](https://spectralops.io) - various services powered by Loco framework
* [Nativish](https://nativi.sh) - app backend powered by Loco framework

[open an issue to add yourself here](https://github.com/loco-rs/loco/issues)


## Contributors ✨

Thanks goes to these wonderful people:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
