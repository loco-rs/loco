+++
title = "Generators"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2024-01-07T21:10:00+00:00
draft = false
weight = 6
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

Generators in `loco` mean code generation tools for improving your workflow. Code generation creates a set of files and code templates based on a predefined set of rules.

Note: this page is a work in progress and is not complete.

## Generate 

Usage: `cargo loco generate [OPTIONS] <COMMAND>

Example:

```sh
$ cargo loco generate --help
```

Options:

`-e, --environment <ENVIRONMENT>` - specify the environment [default: development]
`-h, --help` - print help
`-V, --version` - print version

## Generate commands

- `cargo loco generate scaffold` - Generate a Create/Read/Update/Delete (CRUD) scaffold, which means a migration, model, controller, and tests.
- `cargo loco generate migration` - Generate a new database migration file.
- `cargo loco generate model` - Generate a new model file for defining the data structure of your application, and test file logic.
- `cargo loco generate controller` - Generate a new controller with the given controller name, and test file
- `cargo loco generate task` - Generate a Task based on the given name
- `cargo loco generate scheduler` - Generate a scheduler jobs configuration template
- `cargo loco generate worker` - Generate worker
- `cargo loco generate mailer` - Generate mailer
- `cargo loco generate deployment` - Generate a deployment infrastructure
- `cargo loco generate help` - print this message or the help of the given subcommand(s)

## loco generate scaffold

Generate a CRUD scaffold, model, controller, migration, and tests. 

Usage: `cargo loco generate scaffold [OPTIONS] <NAME> [FIELDS]...`

Arguments:

- `<NAME>` - Name of the thing to generate
- `[FIELDS]...` - Model fields, eg. title:string hits:int

Options that are especially for a controller:

- `-k, --kind <KIND>` - The kind of controller actions to generate [possible values: api, html, htmx]
- `--htmx` - Use HTMX controller actions
- `--html` - Use HTML controller actions
- `--api` - Use API controller actions

This command automatically prepends two model metadata fields:

- Column `created_at`: DateTimeWithTimeZone, not_null
- Column `updated_at`: DateTimeWithTimeZone, not_null

This command automatically generates a default `User` scaffold by default.

Example:

```sh
$ cargo loco generate scaffold item name:string! --api
added: "migration/src/m20241226_224958_items.rs"
injected: "migration/src/lib.rs"
injected: "migration/src/lib.rs"
added: "tests/models/items.rs"
injected: "tests/models/mod.rs"
…
Connecting to Postgres ...
Discovering schema ...
... discovered.
Generating items.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
    > Column `id`: i32, auto_increment, not_null
    > Column `name`: String, not_null
Generating users.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
    > Column `id`: i32, auto_increment, not_null
    > Column `pid`: Uuid, not_null
    > Column `email`: String, not_null, unique
    > Column `password`: String, not_null
    > Column `api_key`: String, not_null, unique
    > Column `name`: String, not_null
    > Column `reset_token`: Option<String>
    > Column `reset_sent_at`: Option<DateTimeWithTimeZone>
    > Column `email_verification_token`: Option<String>
    > Column `email_verification_sent_at`: Option<DateTimeWithTimeZone>
    > Column `email_verified_at`: Option<DateTimeWithTimeZone>
Writing src/models/_entities/items.rs
Writing src/models/_entities/users.rs
Writing src/models/_entities/mod.rs
Writing src/models/_entities/prelude.rs
... Done.
2024-12-26T22:50:06.161928Z  WARN app: loco_rs::boot:  environment=development
added: "src/controllers/item.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/item.rs"
injected: "tests/requests/mod.rs"
* Migration for `item` added! You can now apply it with `$ cargo loco db migrate`.
* A test for model `Items` was added. Run with `cargo test`.
* Controller `Item` was added successfully.
* Tests for controller `Item` was added successfully. Run `cargo test`.
```

## loco generate model

Generates a new model file for defining the data structure of your application, and test file logic.

Usage: `cargo loco generate model [OPTIONS] <NAME> [FIELDS]...`

Arguments:

- `<NAME>` - Name of the thing to generate
- `[FIELDS]...` - Model fields, eg. title:string hits:int. See field type list below.

Options that are especially for a model:

- `-l, --link` - Is it a link table? Use this in many-to-many relations
- `-m, --migration-only` - Generate migration code only. Don't run the migration automatically

This command automatically prepends two model metadata fields:

- Column `created_at`: DateTimeWithTimeZone, not_null
- Column `updated_at`: DateTimeWithTimeZone, not_null

This command automatically generates a default `User` scaffold by default.

Example:

```sh
$ cargo loco generate model item name:string!
added: "migration/src/m20241226_230118_items.rs"
injected: "migration/src/lib.rs"
injected: "migration/src/lib.rs"
added: "tests/models/items.rs"
injected: "tests/models/mod.rs"
…
Connecting to Postgres ...
Discovering schema ...
... discovered.
Generating items.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
    > Column `id`: i32, auto_increment, not_null
    > Column `name`: String, not_null
Generating users.rs
    > Column `created_at`: DateTimeWithTimeZone, not_null
    > Column `updated_at`: DateTimeWithTimeZone, not_null
    > Column `id`: i32, auto_increment, not_null
    > Column `pid`: Uuid, not_null
    > Column `email`: String, not_null, unique
    > Column `password`: String, not_null
    > Column `api_key`: String, not_null, unique
    > Column `name`: String, not_null
    > Column `reset_token`: Option<String>
    > Column `reset_sent_at`: Option<DateTimeWithTimeZone>
    > Column `email_verification_token`: Option<String>
    > Column `email_verification_sent_at`: Option<DateTimeWithTimeZone>
    > Column `email_verified_at`: Option<DateTimeWithTimeZone>
Writing src/models/_entities/items.rs
Writing src/models/_entities/users.rs
Writing src/models/_entities/mod.rs
Writing src/models/_entities/prelude.rs
... Done.
2024-12-26T23:01:25.977481Z  WARN app: loco_rs::boot:  environment=development
* Migration for `item` added! You can now apply it with `$ cargo loco db migrate`.
* A test for model `Items` was added. Run with `cargo test`.
```

## loco generate controller

Generate a new controller with the given controller name, and test file

Usage: `cargo loco generate controller [OPTIONS] <NAME> [ACTIONS]...`

Arguments:

- `<NAME>` - Name of the thing to generate
- `[ACTIONS]...` - Actions

Options that are especially for a controller:

- `-k, --kind <KIND>` - The kind of controller actions to generate [possible values: api, html, htmx]
- `--htmx` - Use HTMX controller actions
- `--html` - Use HTML controller actions
- `--api` - Use API controller actions

If you choose to generate HTML or HTMX, then loco creates a controller:

```sh
$ cargo loco generate controller example --html
added: "src/controllers/example.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
* Controller `Example` was added successfully.
```

If you choose to generate an API, then loco creates a controller and also a test file:

```sh
$ cargo loco generate controller example --api                            
added: "src/controllers/example.rs"
injected: "src/controllers/mod.rs"
injected: "src/app.rs"
added: "tests/requests/example.rs"
injected: "tests/requests/mod.rs"
* Controller `Example` was added successfully.
* Tests for controller `Example` was added successfully. Run `cargo test`.
```

## loco generate migration

Generates a new migration file.

Usage: `cargo loco generate migration [OPTIONS] <NAME>`

Arguments:

- `<NAME>` - Name of the migration to generate

Example:

```sh
$ cargo loco generate migration example            
added: "migration/src/m20241225_200407_example.rs"
injected: "migration/src/lib.rs"
injected: "migration/src/lib.rs"
```

## loco generate task

Generate a Task based on the given name.

Usage: `cargo loco generate task [OPTIONS] <NAME>`

Arguments:

- `<NAME>` - Name of the thing to generate

Example:

```sh
$ cargo loco generate task example
added: "src/tasks/example.rs"
injected: "src/tasks/mod.rs"
injected: "src/app.rs"
added: "tests/tasks/example.rs"
injected: "tests/tasks/mod.rs"
```

## loco generate scheduler

Generate a scheduler jobs configuration template.

Usage: `cargo loco generate scheduler [OPTIONS]`

Example: 

```sh
$ cargo loco generate scheduler
added: "config/scheduler.yaml"
```

The output file is a YAML file which defines scheduler jobs. The file contains an example job that will echo the word "loco" to a file `scheduler.txt` every second.

## loco generate mailer

Generate mailer.

Usage: `cargo loco generate mailer [OPTIONS] <NAME>`

Arguments:

- `<NAME>` - Name of the thing to generate

Example:

```sh
$ cargo loco generate mailer example                                      
added: "src/mailers/example.rs"
injected: "src/mailers/mod.rs"
added: "src/mailers/example/welcome/subject.t"
added: "src/mailers/example/welcome/text.t"
added: "src/mailers/example/welcome/html.t"
```

## loco generate deployment

Generate a deployment infrastructure

Usage: `cargo loco generate deployment [OPTIONS]`

Options:

- TODO: Add flags for different deployment types using Docker, Shuttle, NGINX.

Example with Docker:

```sh
$ cargo loco generate deployment
✔ ❯ Choose your deployment · Docker
added: "dockerfile"
added: ".dockerignore"
```

Example with Shuttle:

```sh
$ cargo loco generate deployment
✔ ❯ Choose your deployment · Shuttle
added: "src/bin/shuttle.rs"
injected: ".cargo/config.toml"
injected: ".cargo/config.toml"
injected: "Cargo.toml"
injected: "Cargo.toml"
added: "Shuttle.toml"
```

Example with NGINX:

```sh
$ cargo loco generate deployment
❯ Choose your deployment · Nginx
added: "nginx/default.conf"
```

## loco generate worker

Generate worker.

Usage: `cargo loco generate worker [OPTIONS] <NAME>`

Arguments:

- `<NAME>` - Name of the thing to generate

Example:

```sh
$ cargo loco generate worker example                                      
added: "src/workers/example.rs"
injected: "src/workers/mod.rs"
injected: "src/app.rs"
added: "tests/workers/example.rs"
injected: "tests/workers/mod.rs"
```

## Field types

Field types for models and scaffolds can be two kinds:

- 1. A specific schema type such as `string` or `int` or `blob`, as described below.
  
- 2. The special type `references` which creates a foreign key relationship between two tables.

Most of the field types come in multiple kinds, depending on the suffix:

- `string` - no special suffix, which means can be null, and can be non-unique.

- `string!` - the special bang suffix, which means must exist.

- `string^` - the special caret suffix means must be unique.

There is one irregular field type:

- `uuid` - must be unique

## Field types that are a specific schema type

Complete list:

- `uuid`
- `uuid_col`, `uuid_col!`
- `string`, `string!`, `string^`
- `text`, `text!`
- `tiny_int`, `tiny_int!`, `tiny_int^`
- `small_int`, `small_int!`, `small_int^`
- `int`, `int!`, `int^`
- `big_int`, `big_int!`, `big_int^`
- `float`, `float!`
- `double`, `double!`
- `decimal`, `decimal!`
- `decimal_len`, `decimal_len!`
- `bool`, `bool!`
- `date`, `date!`
- `ts`, `ts!`
- `tstz`, `tstz!`
- `json`, `json!`
- `jsonb`, `jsonb!`
- `blob`, `blob!`
- `money`, `money!`

For field types, such as for a model or scaffold, you can use the following mapping to understand the schema.

- `uuid` - `uuid_uniq`
- `uuid_col` - `uuid_null`
- `uuid_col!` - `uuid`
- `string` - `string_null`
- `string!` - `string`
- `string^` - `string_uniq`
- `text` - `text_null`
- `text!` - `text`
- `tiny_integer` - `tiny_integer_null`
- `tiny_integer!` - `tiny_integer`
- `tiny_integer^` - `tiny_integer_uniq`
- `small_integer` - `small_integer_null`
- `small_integer!` - `small_integer`
- `small_integer^` - `small_integer_uniq`
- `int` - `integer_null`
- `int!` - `integer`
- `int^` - `integer_uniq`
- `big_int` - `big_integer_null`
- `big_int!` - `big_integer`
- `big_int^` - `big_integer_uniq`
- `float` - `float_null`
- `float!` - `float`
- `double` - `double_null`
- `double!` - `double`
- `decimal` - `decimal_null`
- `decimal!` - `decimal`
- `decimal_len` - `decimal_len_null`
- `decimal_len!` - `decimal_len`
- `bool` - `boolean_null`
- `bool!` - `boolean`
- `tstz` - `timestamp_with_time_zone_null`
- `tstz!` - `timestamp_with_time_zone`
- `date` - `date_null`
- `date` - `date`
- `ts` - `timestamp_null`
- `ts!` - `timestamp`
- `json` - `json_null`
- `json!` - `json`
- `jsonb` - `json_binary_null`
- `jsonb!` - `json_binary`

## Field types that are references

The field type `references` creates a foreign key relationship between two tables.

Example:

```sh
cargo loco generate model item foo:references`
```

The field `foo:references` creates the field `FooId` which references the table `Foos`, and creates a foreign key relationship in the migration file:

```rust
    ForeignKey::create()
        .name("fk-items-foo_ids")
        .from(Items::Table, Items::FooId)
        .to(Foos::Table, Foos::Id)
        .on_delete(ForeignKeyAction::Cascade)
        .on_update(ForeignKeyAction::Cascade)
```

Item:

```sh
cargo loco generate model item foo:references`
```

The field `foo:references:bar` creates the field `FooId` which references the table `Bars`, and creates a foreign key relationship in the migration file:

```rust
ForeignKey::create()
    .name("fk-items-foo_ids")
    .from(Items::Table, Items::FooId)
    .to(Bars::Table, Bars::Id)
    .on_delete(ForeignKeyAction::Cascade)
    .on_update(ForeignKeyAction::Cascade)
```

## Creating A Custom Generator

TODO: Add information akin to Rails Guides https://guides.rubyonrails.org/generators.html

## Creating Generators with Generators

TODO: Add help akin to Rails Guides https://guides.rubyonrails.org/generators.html

## Generator Command Line Options

TODO: Add help akin to Rails Guides https://guides.rubyonrails.org/generators.html
