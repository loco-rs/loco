 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

 # Loco

 #### Loco is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

 ## Quick Start
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco-cli
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

 Now you can create your new app (choose "`SaaS` app").

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)

🚂 Loco app generated successfully in:
myapp
```
<!-- </snip> -->


To configure a database , please run a local postgres database with loco:loco and a db named [insert app]_development.
<!-- <snip id="postgres-run-docker-command" inject_from="yaml" template="sh"> -->
```sh
docker run -d -p 5432:5432 \
  -e POSTGRES_USER=loco \
  -e POSTGRES_DB=myapp_development \
  -e POSTGRES_PASSWORD="loco" \
  postgres:15.3-alpine
```
<!-- </snip> -->


 A more advanced set of `docker-compose.yml` and `Dockerfiles` that include Redis and the `mailtutan` mailer are available for [each starter on GitHub](https://github.com/loco-rs/loco/blob/master/starters/saas/.devcontainer/docker-compose.yml).

 Now `cd` into your `myapp` and start your app:

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

## Project Status
+ Stateless APIs
+ Complete `SaaS` products with user authentication
+ Purpose-built services such as ML inference endpoints
+ Full stack projects with separate frontend project integrated with Loco
+ Hobby projects full-stack with backend and HTML frontend

## Powered by Loco
+ [SpectralOps](https://spectralops.io) - various services powered by Loco
  framework
+ [Nativish](https://nativi.sh) - app backend powered by Loco framework

## Contributors ✨
Thanks goes to these wonderful people:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>