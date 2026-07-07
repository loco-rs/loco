# Fork Notice

Roco is a community fork of Loco.

## Description

Roco is derived from the original Loco framework:
[Loco](https://github.com/loco-rs/loco)

The original framework, project name, documentation, contributor history and the credits belong to the Loco maintainers and contributors. Roco retains to have the Apache 2.0 License and preserve to upstream.

## Reason why this fork exists
Roco exists to provide an active and maintained continuous development environment of Loco that wants to have a Rails inspired Rust web framework with predictable release cycles, public roadmap and dependency maintenance.

## Goals
- Maintain active development and issue triage.
- Keep dependencies and address security vulnerabilities up to date.
- Publish periodical releases with changelogs and migration notes.
- Provide a public roadmap for framework, CLI, generator, and documentation work.
- Improve test coverage and CI reliability before large refactors.
- Secured authentication/authorization layer like devise, omniauth, and pundit.
- Websocket layer like ActionCable, SolidCable.
- Database oriented design of queue and job layer like solid queue and sidekiq.
- Improve compilation time.

## Compatibility
During the transition, some crate names, modules, CLI commands, generated code, and documentation may still use `loco`, `loco_rs`, or `loco-rs`.

Roco will document each breaking rename changes before release.



