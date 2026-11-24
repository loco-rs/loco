---
title: Verify signed webhooks
description: Accept webhooks from GitHub, Stripe or another Loco app only when their HMAC signature matches, with the secret in config.
sidebar:
  order: 26
---

Goal: an endpoint that receives webhooks from a third party (GitHub, Stripe, Shopify, another Loco app) and only runs your handler when the request was signed with the shared secret, so a forged or tampered request never reaches your code.

`SignedWebhook<S>` is an extractor. It reads the raw request body, computes `HMAC-SHA256` with the secret from `config/<env>.yaml`, compares it to the signature header in constant time, and only then hands you the verified bytes. In Rails you would write a `before_action` with `OpenSSL::HMAC.hexdigest` and `ActiveSupport::SecurityUtils.secure_compare`; here the extractor is the `before_action`.

It needs the `auth` feature, which is on by default.

## 1. Put the secret in config

Each source gets an entry under `webhooks:`, keyed by a name you choose. Read the secret from the environment at load time — never `std::env::var` in a handler:

```yaml
# config/development.yaml
webhooks:
  github:
    secret: <%= get_env(name="GITHUB_WEBHOOK_SECRET", default="") %>
  stripe:
    secret: <%= get_env(name="STRIPE_WEBHOOK_SECRET", default="") %>
    scheme: timestamped
    tolerance_secs: 300
```

An empty secret refuses every request: HMAC accepts an empty key, so verifying against one would let anyone sign a payload.

The `scheme` says how the signature is laid out:

| `scheme` | Header (default) | Signs | Used by |
|---|---|---|---|
| `hmac` (default) | `X-Hub-Signature-256: sha256=<hex>` | the body | GitHub, Shopify, most others |
| `timestamped` | `Stripe-Signature: t=<secs>,v1=<hex>` | `"{t}.{body}"`, within `tolerance_secs` | Stripe |
| `loco` | `X-Loco-Signature: sha256=<hex>` + `X-Loco-Timestamp` | `"{timestamp}.{body}"`, within `tolerance_secs` | another Loco app's [notification webhooks](/docs/how-to/send-notifications) |

For `hmac`, `header`, `prefix` and `encoding` cover the variations. Shopify, for example, sends a bare base64 digest:

```yaml
webhooks:
  shopify:
    secret: <%= get_env(name="SHOPIFY_WEBHOOK_SECRET", default="") %>
    header: X-Shopify-Hmac-Sha256
    prefix: ""
    encoding: base64
```

Every key is listed in the [configuration reference](/docs/reference/configuration#webhooks).

## 2. Name the source

A source is a marker type whose `NAME` matches the config key:

```rust
use loco_rs::prelude::*;

pub struct GitHub;

impl WebhookSource for GitHub {
    const NAME: &'static str = "github";
}
```

## 3. Take `SignedWebhook<S>` in the handler

```rust
use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct PullRequestEvent {
    action: String,
}

async fn github(
    State(ctx): State<AppContext>,
    hook: SignedWebhook<GitHub>,
) -> Result<Response> {
    if hook.header("X-GitHub-Event") == Some("pull_request") {
        let event: PullRequestEvent = hook.json()?;
        // hand off to a model method or enqueue a worker with `ctx`
        tracing::info!(action = %event.action, "pull request event");
    }
    format::empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/webhooks")
        .add("/github", post(github))
}
```

`SignedWebhook` consumes the body, so it must be the **last** extractor, and you cannot also take `Json<T>`: call `hook.json::<T>()` instead (400 on invalid JSON), or `hook.body()` for the raw bytes when the provider signs something that is not JSON. `hook.headers()` and `hook.header(name)` give you the event-type and delivery-id headers.

Answer quickly: providers time out and retry. Do real work in a [background job](/docs/how-to/add-worker).

If your app turns on forgery (CSRF) protection, exempt the webhook routes: the provider cannot send your CSRF token, and the signature already proves who sent the request.

## 4. Test it

`webhook::sign` produces the exact headers `verify` accepts, so a request test can send a genuine request without copying the signing logic:

```rust
use loco_rs::{config::Webhook, controller::extractor::webhook::sign};

let config: Webhook = serde_json::from_value(serde_json::json!({ "secret": "test" })).unwrap();
let body = r#"{"action":"opened"}"#;
let mut request = server.post("/webhooks/github").text(body);
for (name, value) in sign(&config, body.as_bytes(), chrono::Utc::now().timestamp()) {
    request = request.add_header(name, value);
}
let response = request.await;
```

Put the same secret in `config/test.yaml`, or pass `ctx.config.webhooks["github"]` to `sign`.

## Verify

| Request | Response |
|---|---|
| correctly signed | your handler runs |
| no signature header, wrong secret, body changed after signing | `401 Unauthorized` (the reason is logged, not returned) |
| `timestamped`/`loco` signature older than `tolerance_secs` | `401 Unauthorized` |
| header present but not parseable (wrong prefix, not hex) | `400 Bad Request` |
| no `webhooks.<name>` entry, or an empty secret | `500`, logged as a configuration error |

To check a GitHub setup end to end, GitHub's documented example must verify: secret `It's a Secret to Everybody`, body `Hello, World!`, header `sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17`.

## Next

- [Send notifications](/docs/how-to/send-notifications) — the sending side, including Loco's own signed webhook channel.
- [Run work in the background](/docs/how-to/add-worker) — do the slow part after answering the provider.
- [Configuration reference](/docs/reference/configuration#webhooks) — every `webhooks:` key.
