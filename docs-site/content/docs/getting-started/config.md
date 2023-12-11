+++
title = "Configuration"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
top = false
+++

Configuration in `loco` lives in `config/` and by default sets up 3 different environments:

```
config/
  development.yaml
  production.yaml
  test.yaml
```

An environment is picked up automatically based on:

- A command line flag: `cargo loco start --environment production`, if not given, fallback to
- `LOCO_ENV` or `RAILS_ENV` or `NODE_ENV`

When nothing is given, the default value is `development`.

The `Loco` framework allows support for custom environments in addition to the default environment. To add a custom environment, create a configuration file with a name matching the environment identifier used in the preceding example.

## Placeholders / variables in config

It is possible to inject values into a configuration file. In this example, we get a port value from the `NODE_PORT` environment variable:

```yaml
# config/development.yaml
# every configuration file is a valid Tera template
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port:  {{/* get_env(name="NODE_PORT", default=3000) */}}
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
  # Out of the box middleware configuration. to disable middleware you can changed the `enable` field to `false` of comment the middleware block
```

The [get_env](https://keats.github.io/tera/docs/#get-env) function is part of the Tera template engine. Refer to the [Tera](https://keats.github.io/tera/docs) docs to see what more you can use.

### Example

Suppose you want to add a 'qa' environment. Create a `qa.yaml` file in the config folder:

```
config/
  development.yaml
  production.yaml
  test.yaml
  qa.yaml
```

To run the application using the 'qa' environment, execute the following command:

```
$ LOCO_ENV=qa cargo loco start
```
