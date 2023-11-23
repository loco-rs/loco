# Mailers

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

## Adding a mailer

Now, you need to define your mailer, in `mailers/auth.rs`, add:

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
