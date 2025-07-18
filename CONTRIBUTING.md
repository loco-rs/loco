# Contributing to Loco

Thank you for taking the time to read this.

The first way to show support is to star our repos :).


Loco is a community driven project. We welcome you to participate, contribute and together build a productivity-first web and api framework in Rust.

## Code of Conduct

This project is follows [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## I have a question

If you have a question to ask, feel free to open an new [discussion](https://github.com/loco-rs/loco/discussions). There are no dumb questions.

## I need a feature

Feature requests from anyone is definitely welcomed! You can open an [issue](https://github.com/loco-rs/loco/issues/new/choose). When you can, illustrate a feature with code, simulated console output, and "make believe" console interactions, so we know what you want and what you expect.

## I want to support

Awesome! The best way to support us is to recommend it to your classmates/colleagues/friends, write blog posts and tutorials on our projects and help out other users in the community.

## I want to join

We are always looking for long-term contributors. If you want to commit longer-term to Loco's open source effort, definitely talk with us!

* From time to time we will make issues clear for newcomers with `mentoring` and `good-first-issue`
* If no issue exist, just open an issue and ask how to help

### Using an example app to test

Our testing grounds is [examples/demo](examples/demo/) which is pointing to the latest local `loco` framework. You can use it to test out an actual app, using a locally modified `loco`.


## Code style

We use `rustfmt`/`cargo fmt`. A few code style options are set in the [.rustfmt.toml](.rustfmt.toml) file, and some of them are not stable yet and require a nightly version of rustfmt.

If you're using rustup, the nightly version of rustfmt can be installed by doing the following:
```sh
rustup component add rustfmt --toolchain nightly
```
And then format your code by running:
```sh
rustup default nightly

cargo fmt --all
cargo fmt --all --manifest-path loco-new/Cargo.toml

cargo clippy --fix --allow-dirty --workspace --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms
cargo clippy --fix --allow-dirty --workspace --all-features --manifest-path loco-new/Cargo.toml -- -D warnings -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms

rustup default stable
```

## Testing

Just clone the project and run `cargo test`.
You can see how we test in [.github/workflows](.github/workflows/)

#### Snapshots
We use [insta](https://github.com/mitsuhiko/insta) for snapshot testing, which helps us detect changes in output formats and behavior. To work with snapshots:

1. Install the insta CLI tool:
```sh
cargo install cargo-insta
```

2. Run tests and review/update snapshots:
```sh
cargo insta test --review
```

For CLI-related changes, we maintain separate snapshots of binary command outputs. To update these CLI snapshots:
```sh
LOCO_CI_MODE=true TRYCMD=overwrite cargo test
```

## Docs

The documentation consists of two main components:

+ The [loco.rs website](https://loco.rs) with its source code available [here](./docs-site/).
+ RustDocs.

To reduce duplication in documentation and examples, we use [snipdoc](https://github.com/kaplanelad/snipdoc). As part of our CI process, we ensure that the documentation remains consistent.

Updating the Documentation
+ Download [snipdoc](https://github.com/kaplanelad/snipdoc).
+ Create the snippet in the [yaml file](./snipdoc.yml) or inline the code.
+ Run `snipdoc run`.

To run the documentation site locally, we use [zola](https://www.getzola.org/) so you'll need to [install](https://www.getzola.org/documentation/getting-started/installation/) it. The documentation site works with zola version `0.19.2` and since zola still has breaking changes, we make no guarantees about other versions.

Running the local preview
+ `cd docs-site`
+ `npm run serve` or `zola serve`

## Open A Pull Request

The most recommended and straightforward method to contribute changes to the project involves forking it on GitHub and subsequently initiating a pull request to propose the integration of your modifications into our repository.

Changes a starters project are not recommended. read more [here](./starters/README.md)

### In Your Pull Request Description, Include:
- References to any bugs fixed by the change
- Informative notes for the reviewer, aiding their comprehension of the necessity for the change or providing insights on how to conduct a more effective review.
- A clear explanation of how you tested your changes.

### Your PR must also:
- be based on the master branch
- adhere to the code [style](#code-style)
- Successfully passes the [test suite](#testing)
