# Loco CLI

Loco CLI is a powerful command-line tool designed to streamline the process of generating Loco websites.

## Running Locally

When working with loco-cli against the local Loco repository, you can utilize the `LOCO_DEBUG_PATH` environment variable to point the generator to a local starter instead of fetching from GitHub.

```sh
cd loco-cli
$ LOCO_DEBUG_PATH=[FULL_PATH]/loco-rs/loco  cargo run new --path /tmp
```

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
