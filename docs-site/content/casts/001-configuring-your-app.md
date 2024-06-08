+++
title = "TEST TEST TEST"
description = "TEST TEST TEST"
date = 2023-12-19T09:19:42+00:00
updated = 2023-12-19T09:19:42+00:00
draft = false
template = "casts/page.html"

[taxonomies]
authors = ["Team Loco"]

[extra]
num = "001"
id = "uAyIcgkqOOk"

+++
<em>preparing for Loco Casts, come back later</em>

To build a Rust app with [Axum session](https://crates.io/crates/axum_session), the first step is to choose your server. In this case, we'll use [loco](https://loco.rs) :)

Start by creating a new project and selecting the `SaaS app` template:

```sh
$ cargo install loco-cli
$ loco new
✔ ❯ App name? · myapp
? ❯ What would you like to build? ›
  lightweight-service (minimal, only controllers and views)
  Rest API (with DB and user auth)
❯ SaaS app (with DB and user auth)
```
