+++
title = "Config"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
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

- A command line flag: `rr start --environment production`, if not given, fallback to
- `RR_ENV` or `RAILS_ENV` or `NODE_ENV`

When nothing is given, the default value is `development`.

The `Loco` framework allows support for custom environments in addition to the default environment. To add a custom environment, create a configuration file with a name matching the environment identifier used in the preceding example.

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
