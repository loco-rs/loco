# RustyRails CLI

RustyRails-CLI is a command-line tool designed to simplify the process of generating a RustyRails website.

## Installation

To install RustyRails CLI, use the following command in your terminal:

```sh
cargo install rustyrails
```

## Usage

### Generate the website

This command generates website in your current working directory

```sh
rustyrails new
```

To generate the website in a different directory run the following command

```sh
rustyrails new /my-work/websites/
```

The change the default folder name use `--folder-name` flag

```sh
rustyrails new --folder-name rustyrails-demo
```
