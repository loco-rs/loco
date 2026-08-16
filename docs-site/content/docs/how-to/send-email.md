+++
title = "Send an email"
description = "Generate a mailer, write templates, and configure SMTP with the right TLS mode."
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 24
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/processing/mailers/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: send a transactional email (welcome message, password reset, notification) from a controller or task, without blocking the request while SMTP does its thing.

A mailer delivers over SMTP in the background, using the same [background worker](@/docs/how-to/add-worker.md) infrastructure — calling a mailer enqueues a `MailerWorker` job (on the `"mailer"` queue) and returns immediately.

## 1. Generate a mailer

```sh
cargo loco generate mailer auth
```

This creates `src/mailers/auth.rs`, adds `pub mod auth;` to `src/mailers/mod.rs`, and scaffolds a `welcome/` template directory:

```
src/
  mailers/
    auth/
      welcome/       <-- one directory per email, holding all its parts
        subject.t
        html.t
        text.t
    auth.rs          <-- mailer definition
```

The generated mailer looks like this:

```rust
#![allow(non_upper_case_globals)]
use loco_rs::prelude::*;
use serde_json::json;

static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");

#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {}
impl AuthMailer {
    pub async fn send_welcome(ctx: &AppContext, to: &str, msg: &str) -> Result<()> {
        Self::mail_template(
            ctx,
            &welcome,
            mailer::Args {
                to: to.to_string(),
                locals: json!({
                  "message": msg,
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
```

A template directory must contain exactly three files — `subject.t`, `html.t`, `text.t` (all Tera templates, rendered against `locals`). Missing any one of them is an error at send time.

## 2. Call it from a controller

```rust
use crate::mailers::auth::AuthMailer;

async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    // .. register the user ..
    AuthMailer::send_welcome(&ctx, &user.email, "Welcome!").await?;
    format::json(())
}
```

`mail`/`mail_template` return as soon as the job is enqueued — actual SMTP delivery happens in a worker process.

## 3. Configure SMTP

```yaml
# config/development.yaml — local mail catcher (e.g. mailtutan, MailHog)
mailer:
  smtp:
    enable: true
    host: localhost
    port: 1025
    secure: false
```

```yaml
# config/production.yaml — provider requiring implicit TLS on port 465
mailer:
  smtp:
    enable: true
    host: smtp.example.com
    port: 465
    tls: implicit # overrides `secure` — see below
    auth:
      user: postmaster@mg.example.com
      password: "<%= get_env(name='SMTP_PASSWORD') %>"
    hello_name: mail.example.com # optional EHLO client id
```

### Picking the right `tls` mode

`tls` is the authoritative setting; when present it **overrides** the legacy `secure` boolean:

| `tls` value | Port | Behavior |
|---|---|---|
| `starttls` | 587 (typical) | Connects in cleartext, then upgrades with `STARTTLS`. This is what `secure: true` used to (and still does) select. |
| `implicit` | 465 (typical) | Connection is encrypted from the first byte (SMTPS). **Required** for providers that only accept implicit TLS — `STARTTLS` will not work against a 465 listener. |
| `none` | — | Cleartext, no TLS. Local sinks (Mailpit, mailtutan) only. |

If `tls` is omitted, the legacy `secure` field still works: `secure: true` → `starttls`, `secure: false` → `none`. If you're setting up a provider that documents "port 465 / SMTPS," set `tls: implicit` explicitly — `secure: true` alone cannot express that mode.

## 4. Set a default from-address or priority

Override `opts()` on your mailer:

```rust
impl Mailer for AuthMailer {
    fn opts() -> MailerOpts {
        MailerOpts {
            from: "Acme <noreply@acme.example>".to_string(),
            reply_to: None,
            priority: 100, // default background-queue priority for mailer jobs
        }
    }
}
```

Mailer jobs enqueue at priority `100` by default (`DEFAULT_MAILER_PRIORITY`) — see [Choose a queue backend](@/docs/how-to/choose-queue-backend.md#priority-queues) for what priority means across queue backends. Raise it if a particular mailer's messages (e.g. password resets) should jump ahead of lower-priority background work.

## 5. CC, BCC, and threading headers

`Args` (passed to `mail_template`) and `Email` (passed to `mail`) both support `cc`, `bcc`, and a `headers` field for threading:

```rust
Self::mail_template(
    ctx,
    &welcome,
    mailer::Args {
        to: user.email.clone(),
        cc: Some("audit@acme.example".to_string()),
        bcc: Some("archive@acme.example".to_string()),
        headers: Some(mailer::EmailHeaders {
            in_reply_to: Some(original_message_id.clone()),
            references: Some(original_message_id.clone()),
            message_id: None,
        }),
        locals: json!({ "name": user.name }),
        ..Default::default()
    },
)
.await?;
```

`EmailHeaders` maps to `References`/`In-Reply-To`/`Message-ID`, useful for grouping notification emails into a single thread in the recipient's mail client.

## 6. Run the mailer worker

Mailer delivery goes through the background worker infrastructure, so a worker process must be running to actually send anything:

```sh
cargo loco start --worker            # dedicated worker process
cargo loco start --server-and-worker # server + worker together
```

## 7. Test without sending real email

Set `stub: true` to capture emails instead of sending them:

```yaml
mailer:
  stub: true
```

If mail is dispatched through a worker, set `workers.mode: ForegroundBlocking` in your test config so the send completes synchronously within the test.

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    request::<App, Migrator, _, _>(|request, ctx| async move {
        // .. call the endpoint that sends an email ..

        with_settings!({ filters => cleanup_email() }, {
            assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        });
    })
    .await;
}
```

`deliveries()` (available under the `testing` feature) reports how many emails were "sent" and their content, so you can assert on both.

## Reference

- All `mailer:` YAML keys: [Configuration reference](@/docs/reference/configuration.md#mailer)
- Worker modes and running a worker process: [Add a background worker](@/docs/how-to/add-worker.md)
