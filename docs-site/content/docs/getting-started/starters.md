+++
title = "Starters"
date = 2021-12-19T08:00:00+00:00
updated = 2021-12-19T08:00:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

Simplify your project setup with Loco's predefined boilerplates, designed to make your development journey smoother. To get started, install our CLI and choose the template that suits your needs.

```sh
cargo install loco-cli
```

Create a starter:

```sh
loco new
✔ ❯ App name? · myapp
? ❯ What would you like to build? ›
❯ lightweight-service (minimal, only controllers and views)
  Rest API (with DB and user auth)
  Saas app (with DB and user auth)
```

## Available Starters

#### Saas Starter

The Saas starter is perfect for projects requiring both a frontend website and a REST API. It comes equipped with:

**Frontend**

- Built on React and Vite (easy to replace with your preferred framework).
- Static middleware that point on your frontend build and includes a fallback index.
- The Tera view engine configured for server side view templates, including i18n configuration. Templates and i18n assets live in `assets/`

**Rest API**

- `ping` and `health` endpoints to check service health. See all endpoint with the following command `cargo loco routes`
- Users table and authentication middleware.
- User model with authentication logic and user registration.
- Forgot password API flow.
- Mailer that sends welcome emails and handles forgot password requests.

#### Rest API Starter

Choose the Rest API starter if you only need a REST API without a frontend. If you change your mind later and decide to serve a frontend, simply enable the `static` middleware and point the configuration to your `frontend` distribution folder.

#### Lightweight Service Starter

Focused on controllers and views (response schema), the Lightweight Service starter is minimalistic. If you require a REST API service without a database, frontend, workers, or other features that Loco provides, this is the ideal choice for you!
