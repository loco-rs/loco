# Loco

## Project Status

- [x] `STAGE 1`: **wide but shallow**. Build a lot of components that make up a Rails-like framework, but only invest 60% in what they should be.
- [x] `STAGE 2`: **operable demo**. Build tooling - CLI, testing, and a demo app to showcase the framework
- [ ] `STAGE 3`: **tooling polish**. Make day to day _development_ operations easy, such as adding models, controllers, logic and tests (by code generation, macros, or helpers)
- [ ] `STAGE 4`: **indepth development**. Make the 60% go to 80% of functionality where needed. Things like documentation (website?), missing API, supporting various deployment scenarios (on docker, other platforms), feature flags to cut down functionality
- [ ] `STAGE 5`: **go wide and expand**. Focus on building various kinds of demo apps

## Starting A New Project

To start a new project, you can use cargo-generate:

```
$ cargo install loco-cli
$ loco new
```

## Getting Started

Set up an alias for convenience:

```
alias rr='cargo run --'
```

```
cd examples/demo
```

1. create a database (through your postres admin app):

`rr_app`

2. migrate + generate entities:

```
$ rr db status
$ rr db migrate
$ rr db entities
```

3. run:

= terminal 1 =

```
$ rr start
```

= terminal 2 =

```
$ rr workers
```

## Principles

**Using the framework**

- Convention over configuration, though with a static language like Rust, this is highly diminished
- Developer happiness (testing-first, CLI, tooling, beautiful code)
- ActiveRecord is a good thing, fat models, slim controllers
- Secure defaults
- Optimize for the solo developer, the "one man framework"
- Take advantage and preserve the Rust x-factor: performance, single-binary deploys, coding tools, community

**Building the frameowrk**

- Use a library, and force it into shape (build some glue code / shims / wrappers), rather building it
- If no choice, build it
- Stick to Rails concepts, way of working, naming when can't decide (naming is hard)

## Tech stack

- [axum](https://github.com/tokio-rs/axum) for serving (controllers, router, requests)
- [seaorm](https://www.sea-ql.org/SeaORM/) for data layer (ORM, ActiveModel, migrations)
- [sidekiq-rs](https://github.com/film42/sidekiq-rs) for background jobs
- mailers, views, tasks - homebrewed
