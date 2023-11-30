# Contributing to Loco

Thank you for taking the time to read this. 

The first way to show support is to star our repos :).


Loco is a community driven project. We welcome you to participate, contribute and together build a productivity-first web and api framework in Rust.

## Code of Conduct

This project is follows CNCF [Code of Conduct](https://github.com/cncf/foundation/blob/main/code-of-conduct.md). By participating, you are expected to uphold this code.

## I have a question

If you have a question to ask, feel free to open an issue. There are no dumb questions.

## I need a feature

Feature requests from anyone is definitely welcomed! You can open an issue. When you can, illustrate a feature with code, simulated console output, and "make believe" console interactions, so we know what you want and what you expect.

## I want to support

Awesome! The best way to support us is to recommend it to your classmates/colleagues/friends, write blog posts and tutorials on our projects and help out other users in the community. 

## I want to join

We are always looking for long-term contributors. If you want to commit longer-term to Loco's open source effort, definitely talk with us!

* From time to time we will make issues clear for newcomers with `mentoring` and `good-first-issue`
* If no issue exist, just open an issue and ask how to help

## I want to setup my machine for development and testing

You would need:

* Rust
* Postgres
* `sea-orm-cli` (see [SeaORM](https://www.sea-ql.org/SeaORM/))

### Testing

Just clone the project and run `cargo test`.
You can see how we test in [.github/workflows](.github/workflows/)

### Using an example app to test

Our testing grounds is [examples/demo](examples/demo/) which is pointing to the latest local `loco` framework. You can use it to test out an actual app, using a locally modified `loco`.

