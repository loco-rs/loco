+++
title = "Starters"
date = 2021-12-19T08:00:00+00:00
updated = 2021-12-19T08:00:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

Simplify your project setup with Loco's predefined boilerplates, designed to make your development journey smoother. To get started, install our CLI and choose the template that suits your needs.

<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Create a starter:

<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Loco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

## Available Starters

### SaaS Starter

The SaaS starter is an all-included set up for projects requiring both a UI and a REST API. For the UI this starter supports a client-side app or classic server-side templates (or a combination).

**UI**

- Frontend starter built on React and Vite (easy to replace with your preferred framework).
- Static middleware that point on your frontend build and includes a fallback index. Alternatively you can configure it for static assets for server-side templates.
- The Tera view engine configured for server-side templates, including i18n configuration. Templates and i18n assets live in `assets/`.

**Rest API**

- `ping` and `health` endpoints to check service health. See all endpoint with the following command `cargo loco routes`
- Users table and authentication middleware.
- User model with authentication logic and user registration.
- Forgot password API flow.
- Mailer that sends welcome emails and handles forgot password requests.

#### Configuring assets for serverside templates

The SaaS starter comes preconfigured for frontend client-side assets. If you want to use server-side template rendering which includes assets such as pictures and styles, you can configure the asset middleware for it:

In your `config/development.yaml`, uncomment the server-side config, and comment the client-side config.

```yaml
    # server-side static assets config
    # for use with the view_engine in initializers/view_engine.rs
    #
    static:
      enable: true
      must_exist: true
      precompressed: false
      folder:
        uri: "/static"
        path: "assets/static"
      fallback: "assets/static/404.html"
    fallback:
      enable: false
    # client side app static config
    # static:
    #   enable: true
    #   must_exist: true
    #   precompressed: false
    #   folder:
    #     uri: "/"
    #     path: "frontend/dist"
    #   fallback: "frontend/dist/index.html"
    # fallback:
    #   enable: false
```


### Rest API Starter

Choose the Rest API starter if you only need a REST API without a frontend. If you change your mind later and decide to serve a frontend, simply enable the `static` middleware and point the configuration to your `frontend` distribution folder.

### Lightweight Service Starter

Focused on controllers and views (response schema), the Lightweight Service starter is minimalistic. If you require a REST API service without a database, frontend, workers, or other features that Loco provides, this is the ideal choice for you!
