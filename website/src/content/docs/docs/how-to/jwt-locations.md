---
title: Configure Where Loco Looks for the JWT
description: Set the JWT token location — Bearer header, query parameter, or cookie — as a single location or a fallback list.
sidebar:
  order: 42
---

Goal: control where the `JWT` and `JWTWithUser<T>` extractors look for the token in an incoming request — the `Authorization` header (default), a query string parameter, a cookie, or a fallback list of several.

This setting is `auth.jwt.location` in your config file. It applies only to the JWT extractors (`auth::JWT`, `auth::JWTWithUser<T>`); it has **no effect** on `auth::ApiToken<T>`, which always reads a `Bearer` header regardless of this config — see [Protect a route with an API key](@/docs/how-to/api-key-auth.md#3-send-the-key-as-a-bearer-token).

If you haven't set up JWT auth yet, start with [Protect a route with JWT](@/docs/how-to/jwt-auth.md); this page only covers the `location` key.

## Default: no configuration needed

If you omit `location` entirely, Loco reads the token from the `Authorization: Bearer <token>` header:

```yaml
auth:
  jwt:
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
    # location omitted => Bearer header
```

## Single location

Set `location` to one map with a `from:` tag. The three variants:

**Bearer header** (equivalent to the default, spelled out explicitly):

```yaml
auth:
  jwt:
    location:
      from: Bearer
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
```

**Query parameter** — reads a named query-string value, e.g. for links that can't carry custom headers (email verification links, WebSocket handshakes):

```yaml
auth:
  jwt:
    location:
      from: Query
      name: token
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
```

```sh
curl 'http://127.0.0.1:5150/api/protected?token=<TOKEN>'
```

**Cookie** — reads a named cookie, e.g. for server-rendered apps using session-style cookies:

```yaml
auth:
  jwt:
    location:
      from: Cookie
      name: auth_token
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
```

```sh
curl 'http://127.0.0.1:5150/api/protected' --cookie 'auth_token=<TOKEN>'
```

## Multiple locations (tried in order)

Set `location` to a YAML list instead of a single map. Loco tries each location in the order listed and uses the first one that yields a token — useful when, say, browser clients send a cookie but API clients send a Bearer header:

```yaml
auth:
  jwt:
    location:
      - from: Cookie
        name: auth_token
      - from: Query
        name: token
      - from: Bearer
    secret: "{{ get_env(name='JWT_SECRET') }}"
    expiration: 604800
```

With this config, a request is accepted if it carries a valid token in the `auth_token` cookie, **or** (if that's absent) a `token` query parameter, **or** (if both are absent) an `Authorization: Bearer` header — checked in that order. If none of the configured locations yields a token, the request is rejected with `401 Unauthorized`.

## Verify it works

For a `Multiple` config like the one above, confirm each location independently:

```sh
# via cookie
curl 'http://127.0.0.1:5150/api/protected' --cookie 'auth_token=<TOKEN>'

# via query parameter
curl 'http://127.0.0.1:5150/api/protected?token=<TOKEN>'

# via Bearer header
curl 'http://127.0.0.1:5150/api/protected' --header 'Authorization: Bearer <TOKEN>'
```

Each should succeed independently; a request with no token in any of the three should return `401 Unauthorized`.

## Related

- [Protect a route with JWT](@/docs/how-to/jwt-auth.md) — the extractors this setting controls, and how to generate tokens.
- [Protect a route with an API key](@/docs/how-to/api-key-auth.md) — a separate, always-Bearer-header mechanism this setting does not affect.
- [Configuration reference](@/docs/reference/configuration.md#auth) — the full `auth.jwt` key table, including `JWTLocation`/`JWTLocationConfig` shapes.
