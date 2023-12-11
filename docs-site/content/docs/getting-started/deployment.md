+++
title = "Deployment"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 5
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

Deployment is super simple in Loco, and this is why this guide is super short. Although **most of the time in developemnt you are using `cargo`** when deploying, you use the **binary that was compiled**, there is no need for `cargo` or Rust on the target server.

To deploy, build your production binary for your relevant server architecture:

```
$ cargo build --release
```

And copy your binary along with your `config/` folder to the server. You can then run `myapp start` on your server.

That's it!

We took special care that **all of your work** is embbedded in a **single** binary, so you need nothing on the server other than that.

## Review your production config

There are a few configuration sections that are important to review and set accordingly when deploying to production:

- Logger:

```yaml
logger:
  level: <your production log level>
```

- Server:

```yaml
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: 3000
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
```

- Database:

```yaml
database:
  # Database connection URI
  uri: postgres://loco:loco@localhost:5432/loco_app
```

- Mailer:

```yaml
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: localhost
```

- Redis:

```
redis:
  # Redis connection URI
  uri: redis://127.0.0.1/
```

- JWT secret:

```yaml
auth:
  # JWT authentication
  jwt:
    # Secret key for token generation and verification
    secret: ...
```

## Generate

Loco offers a deployment template enabling the creation of a deployment infrastructure.

```sh
cargo loco generate deployment
? ❯ Choose your deployment ›
❯ Docker
❯ Shuttle
..
✔ ❯ Choose your deployment · Docker
skipped (exists): "dockerfile"
added: ".dockerignore"
```

Deployment Options:

1. Docker:

- Generates a Dockerfile ready for building and deploying.
- Creates a .dockerignore file.

2. Shuttle:

- Generates a shuttle main function.
- Adds `shuttle-runtime` and `shuttle-axum` as dependencies.
- Adds a bin entrypoint for the deployment.

Choose the option that best fits your deployment needs. Happy deploying!

If you have a preference for deploying on a different cloud, feel free to open a pull request. Your contributions are more than welcome!
