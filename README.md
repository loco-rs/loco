 <div align="center">

   <img src="./docs-site/static/header.svg" alt="Roco logo" />

   <h1>Welcome to Roco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Roco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/roco-rs.svg)](https://crates.io/crates/roco-rs)
   [![docs](https://docs.rs/roco-rs/badge.svg)](https://docs.rs/roco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>


English · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md) · [Español](./README.es.md)


## Fork Notice

This repository is a community fork of [Loco](https://github.com/loco-rs/loco). The original framework, project name, documentation, and contributor history belong to the Loco maintainers and contributors. This fork exists to continue maintenance and experimentation while preserving clear attribution to the upstream project.


## What's Roco?
`Roco` is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Roco refreshing. We do not assume you know Rails.

For a deeper dive into how Roco works, including detailed guides, examples, and API references, check out our [documentation website](https://rsonroco.com).


## Features of Roco:

* `Convention Over Configuration:` Similar to Ruby on Rails, Roco emphasizes simplicity and productivity by reducing the need for boilerplate code. It uses sensible defaults, allowing developers to focus on writing business logic rather than spending time on configuration.

* `Rapid Development:` Aim for high developer productivity, Roco’s design focuses on reducing boilerplate code and providing intuitive APIs, allowing developers to iterate quickly and build prototypes with minimal effort.

* `ORM Integration:` Model your business with robust entities, eliminating the need to write SQL. Define relationships, validation, and custom logic directly on your entities for enhanced maintainability and scalability.

* `Controllers`: Handle web requests parameters, body, validation, and render a response that is content-aware. We use Axum for the best performance, simplicity, and extensibility. Controllers also allow you to easily build middlewares, which can be used to add logic such as authentication, logging, or error handling before passing requests to the main controller actions.

* `Views:` Roco can integrate with templating engines to generate dynamic HTML content from templates.

* `Background Jobs:` Perform compute or I/O intensive jobs in the background with a Redis backed queue, or with threads. Implementing a worker is as simple as implementing a perform function for the Worker trait.

* `Scheduler:` Simplifies the traditional, often cumbersome crontab system, making it easier and more elegant to schedule tasks or shell scripts.

* `Mailers:` A mailer will deliver emails in the background using the existing Roco background worker infrastructure. It will all be seamless for you.

* `Storage:` In Roco Storage, we facilitate working with files through multiple operations. Storage can be in-memory, on disk, or use cloud services such as AWS S3, GCP, and Azure.

* `Cache:` Roco provides an cache layer to improve application performance by storing frequently accessed data.

To see more Roco features, check out our [documentation website](https://rsonroco.com/docs/getting-started/tour/).



## Getting Started
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install roco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Now you can create your new app (choose "`SaaS` app").


<!-- <snip id="roco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ roco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Roco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

 Now `cd` into your `myapp` and start your app:
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo roco start

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
                https://rsonroco.com

listening on port 5150
```
<!-- </snip> -->

## Upstream Loco ecosystem
The following projects are built on top of the upstream Loco Framework:

+ [SpectralOps](https://spectralops.io) - various services powered by Loco
  framework
+ [Nativish](https://nativi.sh) - app backend powered by Loco framework

## Roco Contributors ✨

Thanks go to these wonderful people:

<a href="https://github.com/roco-rs/roco/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=roco-rs/roco" alt="Roco contributors" />
</a>


## Upstream Contributors ✨
Roco builds on the work of the original [Loco maintainers and contributors](https://github.com/loco-rs/loco/graphs/contributors).

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
