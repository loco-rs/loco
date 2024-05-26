<!-- <snip id="readme" inject_from="code" strip_prefix="//!"> -->
 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco</h1>

   <h3>🚂 Loco is "Rust on Rails".</h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

 # Loco

 #### Loco is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

 ## Quick Start
 ```sh
 $ cargo install loco-cli
 $ cargo install sea-orm-cli # Only when DB is needed
 ```

 Now you can create your new app (choose "SaaS app").

 ```sh
 $ loco new
 ✔ ❯ App name? · myapp
 ? ❯ What would you like to build? ›
   lightweight-service (minimal, only controllers and views)
   Rest API (with DB and user auth)
 ❯ SaaS app (with DB and user auth)
 🚂 Loco app generated successfully in:
 myapp
 ```

 <div class="infobox">
 To configure a database , please run a local postgres database with
 <code>loco:loco</code> and a db named is the [insert app]_development.
 </div>

 You can use Docker to run a Postgres instance:

 When generating a starter, the database name incorporates your application
 name and the environment. For instance, if you include `myapp`, the database
 name in the `test.yaml`configuration will be `myapp_test`, and in the
 `development.yaml` configuration, it will be `myapp_development`.



 A more advanced set of `docker-compose.yml` and `Dockerfiles` that include Redis and the `mailtutan` mailer are available for [each starter on GitHub](https://github.com/loco-rs/loco/blob/master/starters/saas/.devcontainer/docker-compose.yml).

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
 You don't have to run things through `cargo` but in development it's highly
 recommended. If you build `--release`, your binary contains everything
 including your code and `cargo` or Rust is not needed. </div>

 ## Project Status
 + Stateless APIs
 + Complete SaaS products with user authentication
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
<!-- </snip> -->