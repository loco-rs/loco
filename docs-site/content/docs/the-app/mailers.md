+++
title = "Mailers"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 18
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

A mailer will deliver emails in the background using the existing `loco` background worker infrastructure. It will all be seamless for you.

## Using mailers

To use an existing mailer, mostly in your controller:

```rust
use crate::{
    mailers::auth::AuthMailer,
}

// in your controllers/auth.rs
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Json<()>> {
    // .. register a user ..
    AuthMailer::send_welcome(&ctx, &user.email).await.unwrap();
}
```

This will enqueue a mail delivery job. The action is instant because the delivery will be performed later in the background.

## Mailer Configuration

Configuration for mailers is done in the `config/[stage].toml` file. Here is the default configuration: 

```yaml
# Mailer Configuration.
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: {{/* get_env(name="MAILER_HOST", default="localhost") */}}
    # SMTP server port
    port: 1025
    # Use secure connection (SSL/TLS).
    secure: false
    # auth:
    #   user:
    #   password:
```

Mailer is done by sending emails to a SMTP server. An example configuration for using sendgrid (choosing the SMTP relay option):

```yaml
# Mailer Configuration.
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: {{/* get_env(name="MAILER_HOST", default="smtp.sendgrid.net") */}}
    # SMTP server port
    port: 465
    # Use secure connection (SSL/TLS).
    secure: true
    auth:
      user: "apikey"
      password: "your-sendgrid-api-key"
```

### Default Email Address

Other than specifying email addresses for every email sending task, you can override a default email address per-mailer.

First, override the `opts` function in the `Mailer` trait, in this example for an `AuthMailer`:

```rust
impl Mailer for AuthMailer {
    fn opts() -> MailerOpts {
        MailerOpts {
            from: // set your from email,
            ..Default::default()
        }
    }
}
```

## Adding a mailer

You can generate a mailer:

```sh
cargo loco generate mailer <mailer name>
```

Or, you can define it manually if you like to see how things work. In `mailers/auth.rs`, add:

```rust
static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
impl AuthMailer {
    /// Sending welcome email the the given user
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_welcome(ctx: &AppContext, _user_id: &str) -> Result<()> {
        Self::mail_template(
            ctx,
            &welcome,
            Args {
                to: "foo@example.com".to_string(),
                locals: json!({
                  "name": "joe"
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
```

Each mailer has an opinionated, predefined folder structure:

```
src/
  mailers/
    auth/
      welcome/      <-- all the parts of an email, all templates
        subject.t
        html.t
        text.t
    auth.rs         <-- mailer definition
```

### Running a mailer
The mailer operates as a background worker, which means you need to run the worker separately to process the jobs. The default startup command `cargo loco start` does not initiate the worker, so you need to run it separately:

To run the worker, use the following command:
```bash
cargo loco start --worker
```

To run both the server and the worker simultaneously, use the following command:
```bash
cargo loco start --server-and-worker
```

## Testing a mailer

For testing mailers integrated into your application, `Loco` offers a straightforward implementation. Refer to the documentation [here](@/docs/testing/mailers.md) for detailed information.
