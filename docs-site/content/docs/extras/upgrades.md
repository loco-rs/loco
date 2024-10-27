+++
title = "Upgrades"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

## What to do when a new Loco version is out?

* Create a clean branch in your code repo.
* Update the Loco version in your main `Cargo.toml`
* Consult with the [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md) to find breaking changes and refactorings you should do (if any).
* Run `cargo loco doctor` inside your project to verify that your app and environment is compatible with the new version

As always, if anything turns wrong, [open an issue](https://github.com/loco-rs/loco/issues) and ask for help.

## Major Loco dependencies

Loco is built on top of great libraries. It's wise to be mindful of their versions in new releases of Loco, and their individual changelogs.

These are the major ones:

* [SeaORM](https://www.sea-ql.org/SeaORM), [CHANGELOG](https://github.com/SeaQL/sea-orm/blob/master/CHANGELOG.md)
* [Axum](https://github.com/tokio-rs/axum), [CHANGELOG](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)

