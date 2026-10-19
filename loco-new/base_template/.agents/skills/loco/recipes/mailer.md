# Recipe: sending email

Never hand-build an SMTP client, and never add `lettre` — Loco ships the mailer
and wires the transport into `AppContext`.

```sh
cargo loco generate mailer auth
```

That creates `src/mailers/auth.rs` plus a template directory per message.

## The mailer

```rust
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::users;

static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");

pub struct AuthMailer {}
impl Mailer for AuthMailer {}

impl AuthMailer {
    /// # Errors
    /// When email sending fails.
    pub async fn send_welcome(ctx: &AppContext, user: &users::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &welcome,
            mailer::Args {
                to: user.email.clone(),
                locals: json!({
                    "name": user.name,
                    "verifyToken": user.email_verification_token,
                    "host": ctx.config.server.full_url(),
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
```

Points that are easy to get wrong:

- `impl Mailer for AuthMailer {}` — the trait is empty; it exists to give you
  `Self::mail_template`.
- Templates are embedded at compile time via `include_dir!`, so the path in
  `include_dir!` is relative to the crate root and the directory must exist at
  build time.
- `#![allow(non_upper_case_globals)]` is needed because the convention names
  these statics in lowercase after the template directory.
- `mailer::Args` is `..Default::default()`-friendly; `to` and `locals` are the
  fields you normally set.
- `locals` is a `serde_json::Value` — those keys are what the template renders.

## Templates

```
src/mailers/auth/welcome/
  subject.t
  html.t
  text.t
```

Each is a Tera template rendered with `locals`:

```
Welcome {{ name }}!

Verify your address: {{ host }}/api/auth/verify/{{ verifyToken }}
```

## Calling it

From a controller — one line, after the model work:

```rust
let user = users::Model::create_with_password(&ctx.db, &params).await?;
AuthMailer::send_welcome(&ctx, &user).await?;
```

For anything slow or non-critical to the response, enqueue instead — send the
mail from inside a `BackgroundWorker` so a flaky SMTP server cannot fail the
user's request. See `background-job.md`.

## Configuration

`config/<env>.yaml` — never `std::env::var` in mailer code:

```yaml
mailer:
  smtp:
    enable: true
    host: localhost
    port: 1025
    secure: false
    # auth:
    #   user: <%= get_env(name="SMTP_USER", default="") %>
    #   password: <%= get_env(name="SMTP_PASSWORD", default="") %>
```

Secrets reach the app through **config-level env interpolation**
(`<%= get_env(...) %>`), which is the sanctioned path. Reading env vars directly
from application code is not.

## In development and tests

Loco ships a stub transport. In `config/test.yaml`:

```yaml
mailer:
  stub: true
```

Then assert on delivered mail in a request test rather than standing up a real
SMTP server:

```rust
let deliveries = ctx.mailer.unwrap().deliveries();
assert_eq!(deliveries.count, 1);
assert!(deliveries.messages[0].contains("expected text"));
```

`Deliveries { count: usize, messages: Vec<String> }` — full rendered messages,
headers included. See `testing.md` for the surrounding test.
