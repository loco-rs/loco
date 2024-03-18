+++
title = "What if Rails was Built on Rust?"
description = "Introducing Loco a Rails-inspired Rust web framework"
date = 2023-11-24T09:19:42+00:00
updated = 2023-11-24T09:19:42+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Team Loco"]

+++

<center>
<img width="150" src="/images/logo.png"/> 


**What if [Rails](https://rubyonrails.org) was built on Rust and not Ruby?**
</center>



Then it would look like this:

```rust
async fn current(
    auth: middleware::auth::Auth,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

pub fn routes() -> Routes {
    Routes::new().prefix("user").add("/current", get(current))
}

```

## Introducing: Loco

Loco is a Rails inspired web framework for Rust. It inlcudes _almost every Rails feature_ with best-effort Rust ergonomics:

* Controllers and routing via [axum](https://github.com/tokio-rs/axum)
* Models, migration, and ActiveRecord via [SeaORM](https://www.sea-ql.org/SeaORM/)
* Views via [serde](https://serde.rs/json.html)
* Seamless, Background jobs via [sidekiq-rs](https://github.com/film42/sidekiq-rs), multi modal: in process, out of process, async via Tokio
* Mailers
* Tasks
* Seeding
* Environment-aware configuration
* Tracing, logging, seamlessly integrated via [tracing](https://docs.rs/tracing)
* Generators via [rrgen](https://github.com/jondot/rrgen)
* Batteries-included authentication (like Rails' `devise`)
* Testing kit, with automatic truncation, fixture seeding, auto migration, snapshotting and redaction

It's full stack for real.

## Why not Rails?

If you're happy with Ruby, use Rails. Don't spend time looking elsewhere because of performance -- Rails and Ruby are good enough.

**But if you love Rust**, you can now build companies like Rubyists have been building for ages -- use Loco.

* You'll get **Rust's safety, strong typing, fantastic concurrency models, and super super stable libraries and ecosystem**. Build once, then forget about it.
* Deployment is copying a **single binary** over to a server.
* You'll be getting **an order of 100,000 requests/sec** without any effort. And 50k requests/sec with database calls. You will never need more than a couple servers. Heck, you can deploy on a Rasberry Pi and be happy..

## The One Person Framework

Inspired by [DHH's approach](https://world.hey.com/dhh/the-one-person-framework-711e6318), Loco's guiding principle is above all:

> The one person framework

From this single guiding principles comes everything else.

For example, one person team, or one person company:


* Has **no time to debate libraries**, tooling, linting rules: strong opinions are welcome. Tell me how I should work.
* **Needs a driving tool** in addition to their brainpower -- that's the Loco CLI. Generate code, operate your project.
* **Needs stability**, anything that breaks is a waste of time, any surprise is a waste of time
* **Needs simplicity** -- don't surprise me
* **Needs a single operability story**. Deploys should be simple. No Kubernetes, no IAC, no preconditions.
* **Needs control**. Send emails and author the emails locally, not on some remote service
* **Needs locality**. Everything that happens in production should first happen in development and locally
* **Needs ad-hocness**. No holy grail ceremonies. Build tasks to run birthday emails to your users, rather than go on a crusade for an "Admin" project.

Loco is the one person framework for **indy hackers, hobbyists, and startups**.

With around **20mb of a deploy binary, and 50k requests/sec** - all you need is a single small/medium server, Postgres or Sqlite and an internet connection. Startups should be cheap!


Get started with [Loco](https://loco.rs) today!
