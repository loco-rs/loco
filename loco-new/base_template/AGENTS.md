# Agent guide for this Loco app

This is a [Loco](https://loco.rs) app — **Rails for Rust**. When you are unsure
how something should work here, the answer is almost always "the way Rails does
it." Where Loco diverges, it is because Rust forced it.

## Read this first

A complete Loco skill ships with this app at **`.claude/skills/loco/`**, matched
to the exact `loco-rs` version in `Cargo.toml`:

| File | What it gives you |
|---|---|
| `.claude/skills/loco/SKILL.md` | start here — the router, `AppContext`, project layout, CLI |
| `.claude/skills/loco/doctrine.md` | what good Loco code looks like; read before writing any |
| `.claude/skills/loco/api-index.md` | every public `loco_rs` symbol, generated from rustdoc — **check here before guessing an API name** |
| `.claude/skills/loco/recipes/` | how to add a model, endpoint, worker, task, mailer, middleware, auth, tests |

If your tool supports Agent Skills, it will load `SKILL.md` automatically. If
not, read it directly — it is a normal markdown file.

## The three rules that prevent most mistakes

1. **Generate, then edit.** `cargo loco generate <thing>` writes the file *and*
   the wiring. Rust has no autoloading; hand-wiring is how "the handler exists
   but 404s" happens.
2. **Use the batteries.** This app already has an ORM, queue, scheduler, mailer,
   task runner, storage, cache, and test harness. Adding a crate for something
   Loco already does is the most common mistake.
3. **Fat model, slim controller.** Domain logic on the model; handlers parse,
   call a model method, and render.

## Before you call it done

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## More

- Docs: <https://loco.rs/docs/>
- Framework agent guide: <https://loco.rs/AGENTS.md>
