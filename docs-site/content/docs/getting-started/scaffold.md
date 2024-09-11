+++
title = "Scaffold"
date = 2024-06-06T08:00:00+00:00
updated = 2021-06-06T08:00:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

Scaffolding is an efficient and speedy method for generating key components of an application. By utilizing scaffolding, you can create models, views, and controllers for a new resource all in one go.


See scaffold command:
<!-- <snip id="scaffold-help-command" inject_from="yaml" action="exec" template="sh"> -->
```sh
Generates a CRUD scaffold, model and controller

Usage: blo-cli generate scaffold [OPTIONS] <NAME> [FIELDS]...

Arguments:
  <NAME>       Name of the thing to generate
  [FIELDS]...  Model fields, eg. title:string hits:int

Options:
  -k, --kind <KIND>                The kind of scaffold to generate [default: api] [possible values: api, html, htmx]
  -e, --environment <ENVIRONMENT>  Specify the environment [default: development]
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- </snip> -->

You can begin by generating a scaffold for the Post resource, which will represent a single blog posting. To accomplish this, open your terminal and enter the following command:
<!-- <snip id="scaffold-post-command" inject_from="yaml" template="sh"> -->
```sh
cargo loco generate scaffold posts name:string title:string content:text
```
<!-- </snip> -->

The scaffold generate command support API, HTML or HTMX by adding `--template` flag to scaffold command.


The scaffold generator will build several files in your application:

| File    | Purpose                                                                                                                                    |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| `migration/src/lib.rs`                     |  Include Post migration.                                                                                |
| `migration/src/m20240606_102031_posts.rs`  | Posts migration.                                                                                        |
| `src/app.rs`                               | Adding Posts to application router.                                                                     |
| `src/controllers/mod.rs`                   | Include the Posts controller.                                                                           |
| `src/controllers/posts.rs`                 | The Posts controller.                                                                                   |
| `tests/requests/posts.rs`                  | Functional testing.                                                                                     |
| `src/models/mod.rs`                        | Including Posts model.                                                                                  |
| `src/models/posts.rs`                      | Posts model,                                                                                            |
| `src/models/_entities/mod.rs`              | Includes Posts Sea-orm entity model.                                                                    |
| `src/models/_entities/posts.rs`            | Sea-orm entity model.                                                                                   |
| `src/views/mod.rs`                         | Including Posts views. only for HTML and HTMX templates.                                                |
| `src/views/posts.rs`                       | Posts template generator. only for HTML and HTMX templates.                                             |
| `assets/views/posts/create.html`           | Create post template. only for HTML and HTMX templates.                                                 |
| `assets/views/posts/edit.html`             | Edit post template. only for HTML and HTMX templates.                                                   |
| `assets/views/posts/edit.html`             | Edit post template. only for HTML and HTMX templates.                                                   |
| `assets/views/posts/list.html`             | List post template. only for HTML and HTMX templates.                                                   |
| `assets/views/posts/show.html`             | Show post template. only for HTML and HTMX templates.                                                   |
                                                                                                  