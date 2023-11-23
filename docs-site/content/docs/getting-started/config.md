+++
title = "Configuration"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
top = false
+++

Configuration in `rustyrails` lives in `config/` and by default sets up 3 different environments:

```
config/
  development.yaml
  production.yaml
  test.yaml
```

An environment is picked up automatically based on:

* A command line flag: `rr start --environment production`, if not given, fallback to
* `RR_ENV` or `RAILS_ENV` or `NODE_ENV` 

When nothing is given, the default value is `development`.
