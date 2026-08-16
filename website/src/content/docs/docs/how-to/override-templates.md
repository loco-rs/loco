---
title: Override a built-in generator template
description: Use `cargo loco generate override` to copy a built-in .t template into your app so you can customize what generators produce.
sidebar:
  order: 61
---

Goal: change what `cargo loco generate <kind>` produces — e.g. add a header to every generated controller, or tweak the React pages a scaffold emits — without forking Loco.

Every generator kind covered in [Using generators](/docs/how-to/use-generators) is driven by `.t` template files baked into the `loco-gen` crate. `cargo loco generate override` copies one of those templates (or a whole folder of them) into your app's own `.loco-templates/` directory; from then on, generation runs read your copy instead of the built-in one.

## 1. List what's overridable

Run `override` with no path to see every template, grouped by generator kind:

```sh
cargo loco generate override
```

This prints a tree of all available templates plus example invocations — it ignores `--info` (a bare invocation always lists).

## 2. Preview a specific file or folder with `--info`

Before copying, check what's under a given path (without actually copying anything) by adding `--info`:

```sh
cargo loco generate override scaffold/frontend --info
```

## 3. Copy one file, a folder, or everything

```sh
# override a single template file
cargo loco generate override scaffold/api/controller.t

# override every template under a folder (e.g. all the scaffold's frontend pages)
cargo loco generate override scaffold/frontend

# override every template in the project
cargo loco generate override .
```

Copied files land under `.loco-templates/` (mirroring the built-in path, e.g. `.loco-templates/scaffold/api/controller.t`) and the command prints each file it copied. If nothing matched the given path, it tells you no templates were found instead of silently no-op'ing.

## 4. Edit your copy

Open the copied `.t` file under `.loco-templates/` and edit it like any other [Tera](https://keats.github.io/tera/) template — it uses the same variables (`name`, `pkg_name`, casing filters like `snake_case`/`pascal_case`, etc.) as the built-in one you copied it from. The next time you run the matching `cargo loco generate <kind> ...`, your local copy is used instead of the built-in template.

## 5. Revert to the built-in template

Delete your local copy — Loco always prefers `.loco-templates/` when it exists, and falls back to the built-in template the moment it doesn't:

```sh
rm .loco-templates/scaffold/api/controller.t
```

## Verify it

```sh
cargo loco generate override scaffold/api/controller.t
# edit .loco-templates/scaffold/api/controller.t
cargo loco generate controller widgets show
```

Confirm the generated `src/controllers/widgets.rs` reflects your edited template (e.g. the header/comment you added), not the stock output.

See the [Generators & field types reference](/docs/reference/generators#override) for the exact `override` CLI shape, and the [CLI reference](/docs/reference/cli#2-4-generate-subcommands) for how `override` fits among the other `generate` subcommands.
