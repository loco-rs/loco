+++
title = "Errors"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++


## Error levels and options

As a reminder, error levels and their logging can be controlled in your `development.yaml`:

### Logger
<!-- <snip id="configuration-logger" inject_from="code" template="yaml"> -->
```yaml
# Application logging configuration
logger:
  # Enable or disable logging.
  enable: true
  # Enable pretty backtrace (sets RUST_BACKTRACE=1)
  pretty_backtrace: true
  # Log level, options: trace, debug, info, warn or error.
  level: debug
  # Define the logging format. options: compact, pretty or json
  format: compact
  # By default the logger has filtering only logs that came from your code or logs that came from `loco` framework. to see all third party libraries
  # Uncomment the line below to override to see all third party libraries you can enable this config and override the logger filters.
  # override_filter: trace
```
<!-- </snip> -->

The most important knobs here are:

* `level` - your standard logging levels. Typically `debug` or `trace` in development. In production choose what you are used to.
* `pretty_backtrace` - provides clear, concise path to the line of code causing the error. use `true` in development and turn off in production. In cases where you are debugging things in production and need some extra hand, you can turn it on and then off when you're done.

### Controller logging

In `server.middlewares` you will find:

```yaml
server:
  middlewares:
    #
    # ...
    #
    # Generating a unique request ID and enhancing logging with additional information such as the start and completion of request processing, latency, status code, and other request details.
    logger:
      # Enable/Disable the middleware.
      enable: true
```

You should enable it to get detailed request errors and a useful `request-id` that can help collate multiple request-scoped errors.


### Database

You have the option of logging live SQL queries, in your `database` section:

```yaml
database:
  # When enabled, the sql query will be logged.
  enable_logging: false
```


## Operating around errors

You'll be mostly looking at your terminal for errors while developing your app, it can look something like this:

```bash
2024-02-xxx DEBUG http-request: tower_http::trace::on_request: started processing request http.method=GET http.uri=/notes http.version=HTTP/1.1 http.user_agent=curl/8.1.2 environment=development request_id=8622e624-9bda-49ce-9730-876f2a8a9a46
2024-02-xxx11T12:19:25.295954Z ERROR http-request: loco_rs::controller: controller_error error.msg=invalid type: string "foo", expected a sequence error.details=JSON(Error("invalid type: string \"foo\", expected a sequence", line: 0, column: 0)) error.chain="" http.method=GET http.uri=/notes http.version=HTTP/1.1 http.user_agent=curl/8.1.2 environment=development request_id=8622e624-9bda-49ce-9730-876f2a8a9a46
```

Usually you can expect the following from errors:

* `error.msg` a `to_string()` version of an error, for operators.
* `error.detail` a debug representation of an error, for developers.
* An error **type** e.g. `controller_error` as the primary message tailored for searching, rather than a verbal error message.
* Errors are logged as _tracing_ events and spans, so that you can build any infrastructure you want to provide custom tracing subscribers. Check out the [prometheus](https://github.com/loco-rs/loco/blob/master/loco-extras/src/initializers/prometheus.rs) example in `loco-extras`.

Notes:

* An _error chain_ was experimented with, but provides little value in practice.
* Errors that an end user sees are a completely different thing. We strive to provide **minimal internal details** about an error for an end user when we know a user can't do anything about an error (e.g. "database offline error"), mostly it will be a generic "Inernal Server Error" on purpose -- for security reasons.

## Producing errors

When you build controllers, you write your handlers to return `Result<impl IntoResponse>`. The `Result` here is a Loco `Result`, which means it also associates a Loco `Error` type.

If you reach out for the Loco `Error` type you can use any of the following as a response:

```rust
Err(Error::string("some custom message"));
Err(Error::msg(other_error)); // turns other_error to its string representation
Err(Error::wrap(other_error));
Err(Error::Unauthorized("some message"))

// or through controller helpers:
unauthorized("some message") // create a full response object, calling Err on a created error
```

