+++
title = "CLI"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++


Create your starter app:

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)

🚂 Loco app generated successfully in:
myapp
```
<!-- </snip> -->

Now `cd` into your app, set up a convenience `rr` alias and try out the various commands:

<!-- <snip id="loco-help-command" inject_from="yaml"> -->
```sh
cargo loco --help
```
<!-- </snip> -->

You can now drive your development through the CLI:

```
$ cargo loco generate model posts
$ cargo loco generate controller posts
$ cargo loco db migrate
$ cargo loco start
```

And running tests or working with Rust is just as you already know:

```
$ cargo build
$ cargo test
```

## Starting your app

To run you app, run:

<!-- <snip id="starting-the-server-command" inject_from="yaml"> -->
```sh
cargo loco start
```
<!-- </snip> -->

## Background workers

Based on your configuration (in `config/`), your workers will know how to operate:

```yaml
workers:
  # requires Redis
  mode: BackgroundQueue

  # can also use:
  # ForegroundBlocking - great for testing
  # BackgroundAsync - for same-process jobs, using tokio async
```

And now, you can run the actual process in various ways:

- `rr start --worker` - run only a worker and process background jobs. This is great for scale. Run one service app with `rr start`, and then run many process based workers with `rr start --worker` distributed on any machine you want.

* `rr start --server-and-worker` - will run both a service and a background worker processor in the same unix process. It uses Tokio for executing background jobs. This is great for those cases when you want to run on a single server without too much of an expense or have constrained resources.

## Getting your app version

Because your app is compiled, and then copied to production, Loco gives you two important operability pieces of information:

* Which version is this app, and which GIT SHA was it built from? `cargo loco version`
* Which Loco version was this app compiled against? `cargo loco --version`

Both version strings are parsable and stable so you can use it in integration scripts, monitoring tools and so on.

You can shape your own custom app versioning scheme by overriding the `app_version` hook in your `src/app.rs` file.


