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
    #[serde(skip)]
    pub slack_url: Option<String>,
}

impl Notifier for DeployPaused {
    fn message(&self) -> Message {
        Message::new(format!("Deploys paused by {}", self.by))
            .body(&self.reason)
            .field("project", &self.project)
            .url(format!("https://ci.example.com/{}", self.project))
    }

    fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
        let mut channels = vec![Channel::webhook(), Channel::email()];
        // A per-project Slack URL from your database, or the default one.
        channels.push(match &self.slack_url {
            Some(url) => Channel::slack_url(url),
            None => Channel::slack(),
        });
        channels
    }
}
```

`Notifier` is in the prelude; `Channel` and `Message` come from `loco_rs::notifications`. The struct must be `Serialize`: its fields are sent as `data` in webhook payloads (fields marked `#[serde(skip)]` are not).

`channels` runs for every notification, so it can depend on the data, as above, or on `ctx` (no Slack outside production, say).

| constructor | destination |
|---|---|
| `Channel::slack()` | `notifications.slack.webhook_url` |
| `Channel::slack_url(url)` | that incoming-webhook URL |
| `Channel::webhook()` | `notifications.webhook` (URL, secret, headers) |
| `Channel::webhook_url(url)` | that URL, unsigned; add `.signed(secret)` and `.header(name, value)` |
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
        slack_url: None,
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
```

Every block is optional. A channel without a destination is **skipped with a warning**, not treated as an error: a default channel whose block is missing or whose URL is empty, or an explicit URL that is empty. So leaving `SLACK_WEBHOOK_URL` unset in development turns Slack off.

Destinations and secrets that come from config are read when the job runs, so the Slack URL and the webhook secret never get written to the queue table. URLs and secrets you pass explicitly (`slack_url`, `.signed(..)`) do go into the job payload. An explicit `webhook_url` never receives the configured secret.

## 5. Run a worker

Deliveries run on the background worker, like mail:

```sh
cargo loco start --server-and-worker
```

`NotificationWorker` is registered automatically. A delivery that fails, whether a connection error or a non-2xx answer such as Slack's `429`, returns an error so the queue retries it with exponential backoff. Only the Postgres and SQLite queue providers retry. On Redis, boot logs a warning and each delivery is tried once. In `BackgroundAsync` mode deliveries run on a spawned task with no retries. In `ForegroundBlocking` mode they run inline.

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

A receiver checks the signature, then rejects old timestamps to stop replays. If the receiver is a Loco app:

```rust
use loco_rs::notifications::webhook;

let ok = webhook::verify_signature(&secret, timestamp, &raw_body, signature);
```

## Test notifiers

Set `stub: true` in `config/test.yaml` (the generator does this) and notifications are **recorded instead of sent**, the way the mailer's `stub: true` records email:

```yaml
# config/test.yaml
notifications:
  stub: true
  slack:
    webhook_url: "https://hooks.slack.com/services/test" # never called
```

While recording is on, `deliver_later` and `deliver_now` render every channel, resolve its destination exactly as the worker would (config defaults, the webhook secret and headers, email recipients), and append the result to a recording on the `AppContext`. Nothing is enqueued and nothing goes over the network. Recording happens before the call returns in every worker mode, so the assertions can come straight after it:

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
        slack_url: None,
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

The recording belongs to the context, so tests that boot their own app do not see each other's deliveries. Channels are skipped while recording just as they are when sending, so a test with an unconfigured `Channel::webhook()` records nothing for it. A delivery whose destination cannot be resolved is counted as failed.

To exercise real sending instead, leave `stub` off and point the channels at a server your test controls. With `workers.mode: ForegroundBlocking` (the test default), `deliver_later` runs the worker inline, so the request has already happened when it returns, and the `Report` reflects what was sent. Email deliveries then go through the mailer, where `mailer.stub: true` captures them as described in [Send an email](/docs/how-to/send-email#7-test-without-sending-real-email).
