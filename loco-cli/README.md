# Loco CLI

Loco-CLI is a command-line tool designed to simplify the process of generating a Loco website.

## Installation

To install Loco CLI, use the following command in your terminal:

```sh
cargo install loco-cli
```

## Usage

### Generate the website

This command generates website in your current working directory

```sh
loco new
```

To generate the website in a different directory run the following command

```sh
loco new /my-work/websites/
```

The change the default folder name use `--folder-name` flag

```sh
loco new --folder-name loco-demo
```
