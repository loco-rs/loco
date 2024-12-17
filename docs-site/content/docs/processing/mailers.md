+++
title = "Mailers"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

A mailer will deliver emails in the background using the existing `loco` background worker infrastructure. It will all be seamless for you.

# Sending email

To use an existing mailer, mostly in your controller:

```rust
use crate::{
    mailers::auth::AuthMailer,
}

// in your controllers/auth.rs
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    // .. register a user ..
    AuthMailer::send_welcome(&ctx, &user.email).await.unwrap();
}
```

This will enqueue a mail delivery job. The action is instant because the delivery will be performed later in the background.

## Configuration

Configuration for mailers is done in the `config/[stage].yaml` file. Here is the default configuration:

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
    port: 587
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

### Using a mail catcher in development

You can use an app like `MailHog` or `mailtutan` (written in Rust):

```
$ cargo install mailtutan
$ mailtutan
listening on smtp://0.0.0.0:1025
listening on http://0.0.0.0:1080
```

This will bring up a local smtp server and a nice UI on `http://localhost:1080` that "catches" and shows emails as they are received.

And then put this in your `development.yaml`:

```yaml
# Mailer Configuration.
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: localhost
    # SMTP server port
    port: 1025
    # Use secure connection (SSL/TLS).
    secure: false
```

Now your mailer workers will send email to the SMTP server at `localhost`.

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

# Testing

Testing emails sent as part of your workflow can be a complex task, requiring validation of various scenarios such as email verification during user registration and checking user password emails. The primary goal is to streamline the testing process by examining the number of emails sent in the workflow, reviewing email content, and allowing for data snapshots.

In `Loco`, we have introduced a stub test email feature. Essentially, emails are not actually sent; instead, we collect information on the number of emails and their contents as part of the testing context.

## Configuration

To enable the stub in your tests, add the following field to the configuration under the mailer section in your YAML file:

```yaml
mailer:
  stub: true
```

Note: If your email sender operates within a [worker](@/docs/processing/workers.md) process, ensure that the worker mode is set to ForegroundBlocking.

Once you have configured the stub, proceed to your unit tests and follow the example below:

## Writing a test

Test Description:

- Create an HTTP request to the endpoint responsible for sending emails as part of your code.
- Retrieve the mailer instance from the context and call the deliveries() function, which contains information about the number of sent emails and their content.

```rust
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    request::<App, Migrator, _, _>(|request, ctx| async move {
        // Create a request for user registration.

        // Now you can call the context mailer and use the deliveries function.
        with_settings!({
            filters => cleanup_email()
        }, {
            assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        });
    })
    .await;
}
```

