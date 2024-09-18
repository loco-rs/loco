# Loco CLI

Loco CLI is a powerful command-line tool designed to streamline the process of generating Loco websites.

## Installation

To install Loco CLI, execute the following command in your terminal:

```sh
cargo install loco-cli
```

## Usage

### Generating a Website

This command generates a website in your current working directory:

```sh
loco new
```

To generate the website in a different directory, use the following command:

```sh
loco new --path /my-work/websites/
```


## Running Locally

When working with loco-cli against the local Loco repository, you can utilize the `STARTERS_LOCAL_PATH` environment variable to point the generator to a local starter instead of fetching from GitHub.

```sh
cd loco-cli
$ STARTERS_LOCAL_PATH=[FULL_PATH]/loco-rs/loco  cargo run new --path /tmp
```

## Starters folder

This CLI depends on a folder with _starters_. Each starter is a folder with a `generator.yaml` in its root.

The `generator.yaml` file describes:

* _Global replacements_: a regex describing things to replace such as a mock app name with a real app name that the user selected.

For example:
```yaml
...
rules:
  - pattern: loco_starter_template
    kind: LibName
    file_patterns:
      - rs
      - toml
      - trycmd
  - pattern: PqRwLF2rhHe8J22oBeHy
    kind: JwtToken
    file_patterns:
      - config/test.yaml
      - config/development.yaml
```

* _Starter options_: some starters can configure based on multiple options: which database to use, which asset pipeline, which kind of background worker configuration. Each starter _declares_ what kind of options it subscribes into and is relevant for it.

The options are picked up in generation, for each option a selection is made for the user to pick.

For example:

```yaml
---
description: SaaS app (with DB and user auth)
options:
  - db
  - bg
  - assets
rules:
    # ...
```

As an example, for the `db` option: `postgres` or `sqlite` is offered as a selection.

The source of truth of _which options_ exist and _which selection for each option_ is based on 2 factors:

1. A set of enums to describe all options (in this project, the CLI)
2. Support of the options and formatting of the configuration: in the main Loco project

Enabling or disabling options are done by:

* Replacing text with a different text (such as configuration value for background worker type)
* Enabling or disabling blocks in the configuration by adding or removing comment blocks, using block markers inside the configuration file (`(block-name-start)`, etc)
