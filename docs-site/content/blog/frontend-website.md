+++
title = "Creating Frontend Website"
description = "Build a REST API quickly with Loco and then follow by building a React frontend app to use it. Learn about generators, configuring asset serving and client-side apps with Loco."
date = 2023-12-14T09:19:42+00:00
updated = 2023-12-14T09:19:42+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Team Loco"]

+++

## Overview

This guide provides a comprehensive walkthrough on using `Loco` to build a Todo list application with a REST API and a React frontend. The steps outlined cover everything from project creation to deployment.

Explore the example repository [here](https://github.com/loco-rs/todo-list-example)

The key steps include:

- Creating a Loco project with the SaaS starter
- Setting up a Vite frontend with React
- Configuring Loco to serve frontend static assets
- Implementing the Notes model/controller in the REST API
- Reloading the server and frontend during development
- Deploying the website to production

## Selecting SaaS Starter

To begin, run the following command to create a new Loco app using the SaaS starter:

```sh
& loco new
✔ ❯ App name? · todolist
✔ ❯ What would you like to build? · SaaS app (with DB and user auth)

🚂 Loco app generated successfully in:
/tmp/todolist
```

Follow the prompts to specify the app name (e.g., todolist) and choose the SaaS app option.

After generating the app, ensure you have the necessary resources by running:

```
$ cd todolist
$ cargo loco doctor
✅ SeaORM CLI is installed
✅ DB connection: success
✅ Redis connection: success
```

Verify that SeaORM CLI is installed, and the database and Redis connections are successful. If any resources fail, refer to the [quick tour guide](@/docs/getting-started/guide.md) for troubleshooting.

Once `cargo loco doctor` shows all checks passed, start the server:

```
$ cargo loco start
   Updating crates.io index
   .
   .
   .

                      ▄     ▀
                                 ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
 ██████  █████   ███ █████   ███ █████   ███ ▀█
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

environment: development
   database: automigrate
     logger: debug
      modes: server

listening on port 5150
```

## Creating the Frontend

For the frontend, we'll use [Vite](https://vitejs.dev/guide/) with React. In the `todolist` folder, run:

```sh
$ npm create vite@latest
Need to install the following packages:
  create-vite@5.1.0
Ok to proceed? (y) y
✔ Project name: … frontend
✔ Select a framework: › React
✔ Select a variant: › JavaScript
```

Follow the prompts to set up the `frontend` as a project name.

Navigate to the frontend folder and install dependencies:

```
$ cd todolist/frontend
$ pnpm install
```

Start the development server:

```sh
$ pnpm dev
```

### Serving Static Assets in Loco

First, move all our rest api endpoint under `/api` prefix. for doing it go to `src/app.rs`. in `routes` hooks function add `.prefix("/api")` to the default routes.
```rust
fn routes() -> AppRoutes {
    AppRoutes::with_default_routes()
        .prefix("/api")
        .add_route(controllers::notes::routes())
}
```

Build the frontend for production:

```sh
pnpm build
```

In the `frontend` folder, a `dist` directory is created. Update the `config/development.yaml` file in the main folder to include a static middleware:

```yaml
server:
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
```

Now, run the Loco server again and you should see frontend app serving via Loco
```sh
$ cargo loco start
```

If you see the default fallback page, you have to disable the fallback middleware. The default fallback takes priority over the static handler, so no static content will be served if it is enabled. You can disable it like so:

```yaml
server:
  middlewares:
    fallback:
      enable: false
    static:
      ...
```

# Developing the UI

Install `react-router-dom`, `react-query` and `axios`

```sh
$ pnpm install react-router-dom react-query axios
```

1. Copy [main.jsx](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/main.jsx) to frontend/src/main.jsx.
2. Copy [App.jsx](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/App.jsx) to frontend/src/App.jsx.
3. Copy [App.css](https://github.com/loco-rs/todo-list-example/blob/main/frontend/src/App.css) to frontend/src/App.css.

Now, run the server `cargo loco start` and the UI pnpm dev in the frontend folder, and start adding your todo list!

## Improve Development

use [cargo-watch](https://crates.io/crates/cargo-watch) for hot reloading the server:

```sh
$ cargo watch --ignore "frontend" -x check -s 'cargo run start'
```

Now, any changes in your Rust code will automatically reload the server, and any changes in your frontend Vite will reload the frontend app.

## Deploy To Production

In the `frontend` folder, run `pnpm build`. After a successful build, go to the Loco server and run `cargo loco start`. Loco will serve the frontend static files directly from the server.

### Prepare Docker Image

Run `cargo loco generate deployment` and select Docker as the deployment type:

```sh
$ cargo loco generate deployment
✔ ❯ Choose your deployment · Docker
added: "dockerfile"
added: ".dockerignore"
```

Loco will add a `Dockerfile` and a `.dockerignore `file. Note that Loco detect the static assent and included them as part of the image

Build the container:

```sh
$ docker build . -t loco-todo-list
```

Now run the container:

```sh
$ docker run -e LOCO_ENV=production -p 5150:5150 loco-todo-list start
```
