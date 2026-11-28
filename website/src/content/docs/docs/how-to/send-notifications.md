---
title: Send notifications
description: Define a notifier once and deliver it to Slack, a signed webhook and email in the background, with retries.
sidebar:
  order: 25
---

Goal: tell people and systems that something happened ("deploy paused by ana", "invoice paid") on several channels at once, with one definition, without the request that triggered it waiting on Slack or failing when a channel is down.

A **notifier** is a struct that holds the event's data and says two things: what the notification reads like, and where it goes. Calling `deliver_later` renders one delivery per channel and enqueues each on the `notifications` queue, where the built-in `NotificationWorker` sends it. Channels are independent: each has its own job and its own retries, so a Slack outage never re-sends the email.

Built-in channels:

| channel | sends |
|---|---|
| Slack | a `mrkdwn` message to a Slack incoming webhook |
| Webhook | a JSON `POST`, optionally signed with HMAC-SHA256 |
| Email | an email through the app's [mailer](/docs/how-to/send-email) |

## 1. Enable the feature and generate a notifier

```sh
cargo loco generate notifier deploy_paused
```

This creates `src/notifiers/deploy_paused.rs` (and `src/notifiers/mod.rs` plus `pub mod notifiers;` in `src/lib.rs` the first time), turns on the `notifications` feature of `loco-rs` in `Cargo.toml`, and adds a `notifications:` block to `config/*.yaml`. Without the generator, enable the feature yourself:

```toml
loco-rs = { version = "*", features = ["notifications"] }
```

The email channel also needs the `mailer` feature, which is on by default.

## 2. Define the notifier

```rust
use loco_rs::notifications::{Channel, Message};
use loco_rs::prelude::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DeployPaused {
    pub project: String,
    pub by: String,
    pub reason: String,
}

impl Notifier for DeployPaused {
    fn message(&self) -> Message {
        Message::new(format!("Deploys paused by {}", self.by))
            .body(&self.reason)
            .field("project", &self.project)
            .url(format!("https://ci.example.com/{}", self.project))
    }

    fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
        vec![Channel::slack(), Channel::webhook(), Channel::email()]
    }
}
```

`Notifier` is in the prelude; `Channel` and `Message` come from `loco_rs::notifications`. The struct must be `Serialize`: its fields are sent as `data` in webhook payloads (fields marked `#[serde(skip)]` are not).

`channels` runs for every notification, so it can depend on the data (each project's own Slack channel, see [per-record destinations](#per-record-destinations)) or on `ctx` (no Slack outside production, say).

| constructor | destination |
|---|---|
| `Channel::slack()` | `notifications.slack.webhook_url` |
| `Channel::slack_url(url)` | that incoming-webhook URL |
| `Channel::slack_for(key)` | the URL your [destinations](#per-record-destinations) return for `key` when the job runs |
| `Channel::webhook()` | `notifications.webhook` (URL, secret, headers) |
| `Channel::webhook_url(url)` | that URL, unsigned; add `.signed(secret)` and `.header(name, value)` |
| `Channel::webhook_for(key)` | the URL, secret and headers your destinations return for `key` when the job runs |
| `Channel::email()` | `notifications.email.to` |
| `Channel::email_to(addr)` | that address; add `.and_to(addr)` and `.from(sender)` |

## 3. Send it

```rust
async fn pause(
    State(ctx): State<AppContext>,
    Json(params): Json<PauseParams>,
) -> Result<Response> {
    // .. pause deploys ..
    DeployPaused {
        project: params.project,
        by: current_user.name,
        reason: params.reason,
    }
    .deliver_later(&ctx)
    .await;
    format::json(())
}
```

`deliver_later` returns a `Report` (`accepted`, `skipped`, `failed`), not a `Result`: a channel that cannot be rendered or enqueued is logged and counted, and never fails the request. `deliver_now` sends inline, skipping the queue and its retries, and returns an error if any channel failed. It is meant for tasks and scripts.

## 4. Configure the channels

```yaml
# config/production.yaml
notifications:
  slack:
    webhook_url: <%= get_env(name="SLACK_WEBHOOK_URL", default="") %>
  webhook:
    url: https://hooks.example.com/loco
    secret: <%= get_env(name="WEBHOOK_SECRET", default="") %>
    headers:
      Authorization: Bearer <%= get_env(name="WEBHOOK_TOKEN", default="") %>
  email:
    to: [ops@example.com]
    from: Alerts <alerts@example.com> # default: the mailer default sender
  max_retries: 5         # default 5
  retry_backoff_secs: 10 # default 10; doubles on every retry, up to an hour
  timeout_secs: 10       # HTTP timeout per attempt, default 10
  stub: false            # true records deliveries instead of sending them; see Test notifiers
  outbound:              # which addresses Slack and webhook deliveries may reach
    allow_private: false # default false; true turns the guard off (development only)
    allow: []            # hosts, IPs and CIDRs allowed anyway, e.g. [localhost, "10.20.0.0/16"]
```

Every block is optional. A channel without a destination is **skipped with a warning**, not treated as an error: a default channel whose block is missing or whose URL is empty, or an explicit URL that is empty. So leaving `SLACK_WEBHOOK_URL` unset in development turns Slack off.

Destinations and secrets that come from config are read when the job runs, so the Slack URL and the webhook secret never get written to the queue table. URLs and secrets you pass explicitly (`slack_url`, `.signed(..)`) do go into the job payload. An explicit `webhook_url` never receives the configured secret. For URLs and secrets stored per record, use `slack_for` and `webhook_for`, below.

## Per-record destinations

A Slack URL or a webhook secret stored on a project row is a credential. Passing it to `slack_url` or `.signed(..)` copies it into the job payload, where it sits in the queue table and goes stale if someone rotates it before the job runs.

`Channel::slack_for(key)` and `Channel::webhook_for(key)` put only `key` in the job. When the job runs, the worker asks your `Destinations` for the URL, secret and headers, so a rotated secret takes effect for jobs already queued. It works the way an ActiveJob argument is a GlobalID that is loaded again when the job performs.

To send `DeployPaused` to each project's own Slack channel instead of the shared one, give it the project's `pid` and use `slack_for` in place of `slack()`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DeployPaused {
    pub project: String,
    pub project_pid: String,
    pub by: String,
    pub reason: String,
}

impl Notifier for DeployPaused {
    // message() as before

    fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
        vec![
            Channel::slack_for(&self.project_pid),
            Channel::webhook(),
            Channel::email(),
        ]
    }
}
```

Then tell Loco how to turn a key into a destination:

```rust
// src/app.rs
use loco_rs::model::ModelError;
use loco_rs::notifications::{self, Destination, Destinations};

struct ProjectHooks;

#[async_trait]
impl Destinations for ProjectHooks {
    async fn resolve(
        &self,
        ctx: &AppContext,
        channel: &str, // "slack" or "webhook"
        key: &str,
    ) -> Result<Option<Destination>> {
        let project = match projects::Model::find_by_pid(&ctx.db, key).await {
            Ok(project) => project,
            // The project was deleted after the job was queued.
            Err(ModelError::EntityNotFound) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        Ok(match channel {
            "slack" => project.slack_url.map(Destination::new),
            _ => project
                .hook_url
                .map(|url| Destination::new(url).signed_opt(project.hook_secret)),
        })
    }
}

impl Hooks for App {
    // ..
    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        notifications::set_destinations(&ctx, ProjectHooks);
        Ok(ctx)
    }
}
```

- `Ok(None)`, or an empty URL, means the destination is gone (the project was deleted, the hook removed). The delivery is logged and dropped, not retried. Map "not found" to `Ok(None)` as above: `find_by_pid` returns `Err(ModelError::EntityNotFound)` for a missing row, and passing that on with `?` would retry a delivery that can never succeed.
- When the delivery runs inline (`deliver_now`, `workers.mode: ForegroundBlocking`, or while recording in tests), a destination that is gone counts as `skipped` in the `Report`, not `accepted`. A queued delivery only finds out when the job runs.
- An error (the database is down) fails the job, and the queue retries it.
- `Destination::header(name, value)` adds webhook headers. A header set on the channel (`Channel::webhook_for(key).header(..)`) wins over one from the destination.
- A `_for` channel is skipped with a warning when no destinations are registered or `key` is empty.

The key is in the job payload, so use an identifier you don't mind being stored there, such as a `pid`. Header names are compared without regard to case, so a channel's `authorization` header replaces the destination's `Authorization` instead of being sent next to it.

### Private addresses are refused

Slack and webhook deliveries refuse to connect to loopback, private, link-local and other non-public addresses: `127.0.0.1`, `10.0.0.0/8`, `192.168.0.0/16`, the cloud metadata address `169.254.169.254`, IPv6 `::1`, `fc00::/7`, `fe80::/10`, and the IPv4-mapped forms of all of them. A Slack or webhook URL that a user saved for their team cannot be used to reach your database, your admin endpoints or your cloud credentials. This is the protection Ruby apps get from the `ssrf_filter` gem.

The check is on the address the connection actually uses. A URL whose host is an IP is checked before sending. A host name is checked every time it resolves, so a name that resolves to a public address when saved and to `127.0.0.1` later (DNS rebinding) is still refused, and so is a redirect to a private address.

Slack and webhook deliveries connect directly and ignore `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY`, even for public hosts, because a proxy would look up the name out of the guard's reach. If your servers can only reach the internet through a proxy, deliveries will fail to connect; a warning is logged at startup when one of those variables is set. Either open direct outbound access for Slack and your webhook hosts, or set `allow_private: true`, which turns the guard off and uses the proxy again.

A refused delivery cannot succeed on retry, so it is logged as a warning and dropped, never retried. When the delivery runs inline (`deliver_now`, `workers.mode: ForegroundBlocking`, or tests), it counts as `failed` in the `Report`, and `deliver_now` returns an error naming the channel. In development, when the receiver runs on your machine, allow it (`cargo loco generate notifier` adds this to `config/development.yaml` and `config/test.yaml`):

```yaml
# config/development.yaml
notifications:
  outbound:
    allow: [localhost, 127.0.0.1]
```

A host name in `allow` matches the URL's host exactly and is allowed whatever it resolves to; an IP or a CIDR range allows those addresses for any host. `allow_private: true` allows everything.

The guard is a reusable building block for any outbound request to a user-supplied URL, in `loco_rs::net` (with the `notifications` or `auth_oauth` feature, which bring the HTTP client):

```rust
use loco_rs::net::OutboundGuard;

let guard = OutboundGuard::default();             // or one read from your config
guard.check_url(&url)?;                           // IP-literal hosts, before sending
let client = guard.apply(reqwest::Client::builder()).build()?; // resolved addresses and redirects
let response = client.get(&url).send().await?;
```

`OutboundGuard::is_allowed(ip)` and `loco_rs::net::is_public(ip)` answer the question for a single address, and `loco_rs::net::is_blocked(&err)` tells a refusal apart from other request errors.

## 5. Run a worker

Deliveries run on the background worker, like mail:

```sh
cargo loco start --server-and-worker
```

`NotificationWorker` is registered automatically. A delivery that fails with a connection error, a timeout or a retryable answer (`408`, `429` or any `5xx`, such as Slack's `429`) returns an error so the queue retries it with exponential backoff. Any other `4xx` (a revoked Slack URL, a `401` from your endpoint) will never succeed, so it is logged as a warning and the job completes without retrying. Only the Postgres and SQLite queue providers retry. On Redis the worker polls the `notifications` queue by default, and each delivery is tried once; boot logs a warning about that. In `BackgroundAsync` mode deliveries run on a spawned task with no retries. In `ForegroundBlocking` mode they run inline.

## Customize payloads

Each channel has a default rendering of the `Message`, and each one can be overridden on the notifier:

```rust
impl Notifier for DeployPaused {
    // ..

    // Slack Block Kit instead of the default `text` message.
    fn slack_payload(&self, message: &Message) -> serde_json::Value {
        serde_json::json!({
            "text": message.title,
            "blocks": [{
                "type": "section",
                "text": { "type": "mrkdwn", "text": format!("*{}*", message.title) }
            }]
        })
    }

    // Reshape or extend the webhook body.
    fn webhook_payload(&self, mut payload: serde_json::Value) -> serde_json::Value {
        payload["project"] = self.project.clone().into();
        payload
    }

    // Render mailer templates (subject.t, text.t, html.t) with the notifier as locals.
    fn email(&self, _message: &Message) -> Result<loco_rs::notifications::EmailContent> {
        loco_rs::notifications::EmailContent::from_template(&deploy_paused_email, self)
    }

    // Webhook event name; defaults to the type name in snake_case.
    fn event() -> String {
        "deploy.paused".to_string()
    }
}
```

The default Slack message escapes `&`, `<` and `>` in the title, body and fields, so user text cannot inject `<!channel>` mentions. If you build your own payload, escape user text with `loco_rs::notifications::slack::escape`.

## Webhook format and signatures

The default webhook body:

```json
{
  "id": "4b0c…",
  "event": "deploy_paused",
  "created_at": "2026-10-24T09:30:00+00:00",
  "title": "Deploys paused by ana",
  "body": "release freeze",
  "url": "https://ci.example.com/web",
  "fields": [{ "name": "project", "value": "web" }],
  "data": { "project": "web", "by": "ana", "reason": "release freeze" }
}
```

Headers: `X-Loco-Event` (the event name), `X-Loco-Delivery` (the `id`, which stays the same across retries, so receivers can deduplicate on it), and `X-Loco-Timestamp` (the attempt time, in Unix seconds). When the channel has a secret it also sends `X-Loco-Signature: sha256=<hex>`, an HMAC-SHA256 over `"{timestamp}.{raw body}"`.

A receiver checks the signature, then rejects old timestamps to stop replays. If the receiver is a Loco app, give the sender an entry with `scheme: loco` and take `SignedWebhook<S>` in the handler; it does both checks before your code runs:

```yaml
# receiver's config/<env>.yaml
webhooks:
  ci:
    secret: <%= get_env(name="CI_WEBHOOK_SECRET", default="") %>
    scheme: loco
```

```rust
use loco_rs::prelude::*;

pub struct Ci;
impl WebhookSource for Ci {
    const NAME: &'static str = "ci";
}

async fn ci(hook: SignedWebhook<Ci>) -> Result<Response> {
    tracing::info!(event = ?hook.header("X-Loco-Event"), "notification received");
    format::empty()
}
```

See [Verify signed webhooks](/docs/how-to/verify-webhooks) for the details.

## Test notifiers

Set `stub: true` in `config/test.yaml` (the generator does this) and notifications are **recorded instead of sent**, the way the mailer's `stub: true` records email:

```yaml
# config/test.yaml
notifications:
  stub: true
  slack:
    webhook_url: "https://hooks.slack.com/services/test" # never called
```

While recording is on, `deliver_later` and `deliver_now` render every channel, resolve its destination exactly as the worker would (config defaults, the webhook secret and headers, email recipients, your `Destinations` for `_for` channels), and append the result to a recording on the `AppContext`. Nothing is enqueued and nothing goes over the network. Recording happens before the call returns in every worker mode, so the assertions can come straight after it:

```rust
use loco_rs::notifications::testing;

#[tokio::test]
async fn pausing_notifies_slack() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;

    DeployPaused {
        project: "web".into(),
        by: "ana".into(),
        reason: "release freeze".into(),
    }
    .deliver_later(ctx)
    .await;

    let sent = testing::deliveries(ctx);
    let slack: Vec<_> = sent.iter().filter(|d| d.channel == "slack").collect();
    assert_eq!(slack.len(), 1);
    assert!(slack[0].text().contains("Deploys paused by ana"));
    testing::clear(ctx);
}
```

Each `Recorded` delivery has:

| field | holds |
|---|---|
| `channel` | `slack`, `webhook` or `email` |
| `event`, `id` | the event name and delivery ID, as sent in `X-Loco-Event` and `X-Loco-Delivery` |
| `to` | the resolved Slack or webhook URL, or the email recipients |
| `payload` | the Slack or webhook JSON body, or `{from, subject, text, html}` for email |
| `headers`, `signed` | a webhook's extra headers, and whether it would be signed |

`text()` returns the readable part of any channel (Slack `text`, webhook `title` and `body`, or email subject and text), for assertions that should not depend on the channel.

The API, in `loco_rs::notifications::testing`:

- `deliveries(&ctx)`: everything recorded so far, oldest first.
- `clear(&ctx)`: forget the recording and keep recording.
- `record(&ctx)`: turn recording on for a context whose config does not set `stub`.
- `is_recording(&ctx)`: whether recording is on.

The recording belongs to the context, so tests that boot their own app do not see each other's deliveries. Channels are skipped while recording just as they are when sending, so a test with an unconfigured `Channel::webhook()` records nothing for it. A delivery whose destination cannot be resolved is counted as failed; a `_for` key your `Destinations` reports gone is counted as skipped and not recorded.

To exercise real sending instead, leave `stub` off and point the channels at a server your test controls. A test server listens on `127.0.0.1`, which the [outbound guard](#private-addresses-are-refused) refuses, so allow it in `config/test.yaml` (`cargo loco generate notifier` adds this for you):

```yaml
# config/test.yaml
notifications:
  outbound:
    allow: [localhost, 127.0.0.1]
```

Without it, deliveries to the test server are logged as refused and dropped, and the server sees nothing. With `workers.mode: ForegroundBlocking` (the test default), `deliver_later` runs the worker inline, so the request has already happened when it returns, and the `Report` reflects what was sent. Email deliveries then go through the mailer, where `mailer.stub: true` captures them as described in [Send an email](/docs/how-to/send-email#7-test-without-sending-real-email).
