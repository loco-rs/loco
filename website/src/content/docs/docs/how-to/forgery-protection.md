---
title: Protect Forms from Forgery
description: "Refuse cross-site form posts with the forgery_protection middleware: Fetch Metadata checks, or Rails-style authenticity tokens."
sidebar:
  order: 45
---

Goal: stop another site from making a signed-in user's browser submit a form
to your app (cross-site request forgery). This is Rails'
`protect_from_forgery`.

The `forgery_protection` middleware checks every `POST`, `PUT`, `PATCH` and
`DELETE`. `GET`, `HEAD`, `OPTIONS` and `TRACE` are never checked, so keep
those free of side effects. A refused request gets `403`.

## 1. Turn it on

The server-rendered starter already has this. Elsewhere, add it to
`config/<env>.yaml`:

```yaml
server:
  middlewares:
    forgery_protection:
      enable: true
```

This is `mode: header`. It reads the headers every current browser sends.
`Sec-Fetch-Site` must be `same-origin` or `none`; `same-site` (a sibling
subdomain) is refused. When a browser does not send it, `Origin` must match
`Host`, ignoring host case and a default port. Forms and `fetch` calls need
no changes.

Behind a reverse proxy, pass the browser's `Host` header through, or list
your public origin in `trusted_origins`. Otherwise every post from an older
browser is refused.

A request with neither header did not come from a web page (curl, a script, a
server-to-server call), and it passes. Those clients cannot be tricked into
sending a user's cookies.

## 2. Exempt what is not a browser form

Webhooks and bearer-token APIs are posted to by other sites on purpose:

```yaml
server:
  middlewares:
    forgery_protection:
      enable: true
      exempt:
        - /api/webhooks/
      trusted_origins:
        - https://admin.example.com
```

`exempt` takes path prefixes. `trusted_origins` takes origins that may post
across sites, such as a second front end you own. Both apply in either mode.

## 3. Optional: authenticity tokens

`mode: token` is the classic Rails scheme. It needs the `auth_session`
feature, because the token is kept in a signed cookie:

```toml
loco-rs = { version = "*", features = ["auth_session"] }
```

```yaml
server:
  middlewares:
    forgery_protection:
      enable: true
      mode: token
```

Put the hidden field in every form:

```html
<form method="post" action="/posts">
  {{ csrf_field() }}
  <input name="title">
  <button>Save</button>
</form>
```

For `fetch` or htmx, put the meta tags in your layout's `<head>` and send the
value as `X-CSRF-Token`:

```html
<head>
  {{ csrf_meta_tags() }}
</head>
<script>
  const token = document.querySelector('meta[name="csrf-token"]').content;
  fetch("/posts", { method: "POST", headers: { "X-CSRF-Token": token } });
</script>
```

`{{ csrf_token() }}` gives the bare value. A handler that renders without
Tera takes the `CsrfToken` extractor from the prelude:

```rust
use loco_rs::prelude::*;

async fn token(token: CsrfToken) -> Result<Response> {
    format::json(serde_json::json!({ "token": token.masked() }))
}
```

The value is masked on every render, so it differs from page to page. The
cookie is only set on a response that rendered the token. In production, boot
fails unless `auth.session.secret` is set.

Token mode runs the `header` check first, the way Rails checks the origin
as well as the token, so a cross-site post is refused even when it carries a
token. When cookies are `Secure` (an `https://` `server.host`, or
`auth.session.secure: true`), the cookie is named `__Host-loco_csrf`, which a
sibling subdomain cannot set.

The token is not tied to the session and is not reset on sign-in. With the
checks above, that is safe on `https`; on plain `http` anyone on the network
can set cookies anyway.

To find the form field, the middleware reads the body of a form or multipart
post, up to the app-wide `limit_payload`. That happens before any per-route
body limit, so a larger upload must send the token in `X-CSRF-Token` (read
it from the meta tag) or go under an `exempt` path. A body over the limit
gets `413`.

## Verify

```sh
# Same-origin form post: passes.
curl -i -X POST localhost:5150/posts -H 'sec-fetch-site: same-origin'
# Cross-site: 403 with "cross_site_request".
curl -i -X POST localhost:5150/posts -H 'sec-fetch-site: cross-site'
```

In token mode, a post without the field or header gets `403` with
`invalid_authenticity_token`.

## Next

- [Middleware catalog](/docs/reference/middleware/#10-forgery_protection) lists every knob.
- [Render views](/docs/how-to/render-views/) covers the Tera templates the
  token functions run in.
