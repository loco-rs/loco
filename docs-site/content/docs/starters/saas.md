+++
title = "SaaS <label>Auth</label> <label>DB</label> <label>JS</label> <label>SSR</label>"
date = 2021-12-19T08:00:00+00:00
updated = 2021-12-19T08:00:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

The Saas starter is an all-included set up for projects requiring both a UI and a REST API. For the UI this starter supports a client-side app or classic server-side templates (or a combination).

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

## Configuring assets for serverside templates

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
    # client side app static config
    # static:
    #   enable: true
    #   must_exist: true
    #   precompressed: false
    #   folder:
    #     uri: "/"
    #     path: "frontend/dist"
    #   fallback: "frontend/dist/index.html"
```
