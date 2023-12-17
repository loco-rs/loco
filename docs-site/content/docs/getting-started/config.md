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

## Settings

The configuration files contain knobs to set up your Loco app. You can also have your custom settings, with the `settings:` section.


```yaml
# in config/development.yaml
# add the `settings:` section
settings:
  allow_list:
    - google.com
    - apple.com

logger:
  # ...
```

These setting will appear in `ctx.config.settings` as `serde_json::Value`. You can create your strongly typed settings by adding a struct:

```rust
// put this in src/common/settings.rs
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub allow_list: Option<Vec<String>>,
}

impl Settings {
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(value.clone())?)
    }
}
```

Then, you can access settings from anywhere like this:


```rust
// in controllers, workers, tasks, or elsewhere,
// as long as you have access to AppContext (here: `ctx`)

if let Some(settings) = &ctx.config.settings {
    let settings = common::settings::Settings::from_json(settings)?;
    println!("allow list: {:?}", settings.allow_list);
}
```

## Logger

Other than the commented fields in the `logger:` section on your YAML file, here's some more context:

* `logger.pretty_backtrace` - will display colorful backtrace without noise for great development experience. Note that this forcefully sets `RUST_BACKTRACE=1` into the process' env, which enables a (costly) backtrace capture on specific errors. Enable this in development, disable it in production. When needed in production, use `RUST_BACKTRACE=1` ad-hoc in the command line to show it.

