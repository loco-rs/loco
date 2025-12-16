+++
title = "Deployment"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

Deployment is super simple in Loco, and this is why this guide is super short. Although **most of the time in development you are using `cargo`** when deploying, you use the **binary that was compiled**, there is no need for `cargo` or Rust on the target server.

## How to Deploy
First, check your Cargo.toml to see your application name:
```toml
[package]
name = "myapp" # This is your binary name
version = "0.1.0"
```

build your production binary for your relevant server architecture:

<!-- <snip id="build-command" inject_from="yaml" template="sh"> -->
```sh
cargo build --release
```
<!-- </snip>-->

And copy your binary along with your `config/` folder to the server. You can then run `myapp start` on your server.

```sh
# The binary is located in ./target/release/ after building
./target/release/myapp start
```

That's it!

We took special care that **all of your work** is embbedded in a **single** binary, so you need nothing on the server other than that.

## Review your production config

There are a few configuration sections that are important to review and set accordingly when deploying to production:

- Logger:

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
<!-- </snip>-->
 

- Server:
<!-- <snip id="configuration-server" inject_from="code" template="yaml"> -->
```yaml
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: {{ get_env(name="NODE_PORT", default=5150) }}
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
```
<!-- </snip>-->


- Database:
<!-- <snip id="configuration-database" inject_from="code" template="yaml"> -->
```yaml
database:
  # Database connection URI
  uri: {{get_env(name="DATABASE_URL", default="postgres://loco:loco@localhost:5432/loco_app")}}
  # When enabled, the sql query will be logged.
  enable_logging: false
  # Set the timeout duration when acquiring a connection.
  connect_timeout: 500
  # Set the idle duration before closing a connection.
  idle_timeout: 500
  # Minimum number of connections for a pool.
  min_connections: 1
  # Maximum number of connections for a pool.
  max_connections: 1
  # Run migration up when application loaded
  auto_migrate: true
  # Truncate database when application loaded. This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_truncate: false
  # Recreating schema when application loaded.  This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_recreate: false
```
<!-- </snip>-->


- Mailer:
<!-- <snip id="configuration-mailer" inject_from="code" template="yaml"> -->
```yaml
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: {{ get_env(name="MAILER_HOST", default="localhost") }}
    # SMTP server port
    port: 1025
    # Use secure connection (SSL/TLS).
    secure: false
    # auth:
    #   user:
    #   password:
```
<!-- </snip>-->

- Queue:
<!-- <snip id="configuration-queue" inject_from="code" template="yaml"> -->
```yaml
queue:
  kind: Redis
  # Redis connection URI
  uri: {{ get_env(name="REDIS_URL", default="redis://127.0.0.1") }}
  # Dangerously flush all data in Redis on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
```
<!-- </snip>-->

- JWT secret:
<!-- <snip id="configuration-auth" inject_from="code" template="yaml"> -->
```yaml
auth:
  # JWT authentication
  jwt:
    # Secret key for token generation and verification
    secret: PqRwLF2rhHe8J22oBeHy
    # Token expiration time in seconds
    expiration: 604800 # 7 days
```
<!-- </snip>-->

## Running `loco doctor`

You can run `loco doctor` in your server to check the connection health of your environment. 

```sh
$ myapp doctor --production
```

## Generate

Loco offers a deployment template enabling the creation of a deployment infrastructure.

```sh
$ cargo loco generate deployment --help
Generate a deployment infrastructure

Usage: myapp-cli generate deployment [OPTIONS] <KIND>

Arguments:
  <KIND>  [possible values: docker, shuttle, nginx]
```

<!-- <snip id="generate-deployment-command" inject_from="yaml" template="sh"> -->

```sh
cargo loco generate deployment docker

added: "Dockerfile"
added: ".dockerignore"
* Dockerfile generated successfully.
* Dockerignore generated successfully
```

<!-- </snip>-->

Deployment Options:

1. Docker:

- Generates a Dockerfile ready for building and deploying.
- Creates a .dockerignore file.

2. Shuttle:

- Generates a shuttle main function.
- Adds `shuttle-runtime` and `shuttle-axum` as dependencies.
- Adds a bin entrypoint for the deployment.

3. Nginx:

- Generates a nginx configuration file for reverse proxying.

Choose the option that best fits your deployment needs. Happy deploying!

If you have a preference for deploying on a different cloud, feel free to open a pull request. Your contributions are more than welcome!
