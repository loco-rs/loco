---
title: FAQ
description: Answers to frequently asked questions.
sidebar:
  order: 2
---

<details>
<summary>How can I automatically reload code?</summary>

Try [cargo watchexec](https://crates.io/crates/watchexec):

```
$ watchexec --notify -r -- cargo loco start
```

Or [bacon](https://github.com/Canop/bacon)

```
$ bacon run
```

</details>
<br/>
<details>
<summary>Do I have to have `cargo` to run tasks or other things?</summary>
You don't have to run things through `cargo` but in development it's highly recommended. If you build `--release`, your binary contains everything including your code and `cargo` or Rust is not needed.
</details>

<br/>

<details>
<summary>Is this production ready?</summary>

Loco reached 1.0 in July 2026, marking its first stable release, but its roots are older. It's almost a rewrite of `Hyperstackjs.io`, and Hyperstack is based on an internal Rails-like framework which is production ready.

Most of Loco is glue code around Axum, SeaORM, and other stable frameworks, so you can consider that.

As with any framework, evaluate Loco against your application's requirements and report issues if they arise.

</details>

<br/>
<details>
<summary>Adding Custom Middleware in Loco</summary>
Loco is compatible with Axum middlewares. Simply implement `FromRequestParts` in your custom struct and integrate it within your controller.
</details>

<br/>

<details>
<summary>Injecting Custom State or Layers in Loco?</summary>
Yes, you can achieve this by implementing `Hooks::after_routes`. This hook receive Axum routers that Loco has already built, allowing you to seamlessly add any available Axum functions that suit your needs.

If you need your routes or (404) fallback handler to be affected by loco's middleware, you can add them in `Hooks::before_routes` which is called before the middleware is installed.
</details>

<br/>
