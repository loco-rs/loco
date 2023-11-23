+++
title = "CLI"
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

Create your starter app:

```rust
$ cargo install rustyrails-cli
$ rustyrails new
< follow the guide >
```


Now `cd` into your app, set up a convenience `rr` alias and try out the various commands:

```
$ cd myapp
$ alias rr='cargo run --'
$ rr --help
```

You can now drive your development through the CLI:

```
$ rr generate model posts
$ rr generate controller posts
$ rr db migrate
$ rr start
```

And running tests or working with Rust is just as you already know:

```
$ cargo build
$ cargo test
```

## Starting your app
To run you app, run:

```
$ rr start
```

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

* `rr start --worker` - run only a worker and process background jobs. This is great for scale. Run one service app with `rr start`, and then run many process based workers with `rr start --worker` distributed on any machine you want.
* `rr start --server-and-worker` - will run both a service and a background worker processor in the same unix process. It uses Tokio for executing background jobs. This is great for those cases when you want to run on a single server without too much of an expense or have constrained resources.

