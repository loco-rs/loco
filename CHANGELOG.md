# Changelog

## vNext

* Adding compilation mode in the banner [https://github.com/loco-rs/loco/pull/127](https://github.com/loco-rs/loco/pull/127)
* Support shuttle deployment generator [https://github.com/loco-rs/loco/pull/124](https://github.com/loco-rs/loco/pull/124)

## v0.1.6

* refactor: local settings are now `<env>.local.yaml` and available for all environments, for example you can add a local `test.local.yaml` and `development.local.yaml`
* refactor: removed `config-rs` and now doing config loading by ourselves.
* fix: email template rendering will not escape URLs
* Config with variables: It is now possible to use [tera](https://keats.github.io/tera) templates in config YAML files

Example of pulling a port from environment:

```yaml
server:
  port: {{ get_env(name="NODE_PORT", default=3000) }}
```

It is possible to use any `tera` templating constructs such as loops, conditionals, etc. inside YAML configuration files.

* Mailer: expose `stub` in non-test

* `Hooks::before_run` with a default blank implementation. You can now code some custom loading of resources or other things before the app runs
* an LLM inference example, text generation in Rust, using an API (`examples/inference`)
* Loco starters version & create release script [https://github.com/loco-rs/loco/pull/110](https://github.com/loco-rs/loco/pull/110)
* Configure Cors middleware [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
* `Hooks::after_routes` Invoke this function after the Loco routers have been constructed. This function enables you to configure custom Axum logics, such as layers, that are compatible with Axum. [https://github.com/loco-rs/loco/pull/114](https://github.com/loco-rs/loco/pull/114)
* Adding docker deployment generator [https://github.com/loco-rs/loco/pull/119](https://github.com/loco-rs/loco/pull/119)

DOCS:
* Remove duplicated docs in auth section
* FAQ docs: [https://github.com/loco-rs/loco/pull/116](https://github.com/loco-rs/loco/pull/116)

ENHANCEMENTS:
* Remove unused libs: [https://github.com/loco-rs/loco/pull/106](https://github.com/loco-rs/loco/pull/106)
* turn off default features in tokio [https://github.com/loco-rs/loco/pull/118](https://github.com/loco-rs/loco/pull/118)

## 0.1.5

NEW FEATURES
* `format:html` [https://github.com/loco-rs/loco/issues/74](https://github.com/loco-rs/loco/issues/74)
* Create a stateless HTML starter [https://github.com/loco-rs/loco/pull/100](https://github.com/loco-rs/loco/pull/100)
* Added worker generator + adding a way to test workers [https://github.com/loco-rs/loco/pull/92](https://github.com/loco-rs/loco/pull/92)

ENHANCEMENTS:
* CI: allows cargo cli run on fork prs [https://github.com/loco-rs/loco/pull/96](https://github.com/loco-rs/loco/pull/96)

