//! Email deliveries through the app's mailer (`ctx.mailer`).

use include_dir::Dir;
use serde::{Deserialize, Serialize};

use super::Message;
use crate::{
    app::AppContext,
    mailer::{template::Template, MultiEmail, DEFAULT_FROM_SENDER},
    Error, Result,
};

/// The rendered parts of a notification email.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailContent {
    pub subject: String,
    pub text: String,
    pub html: String,
}

impl EmailContent {
    /// Plain email built from the message: the title as subject, then the
    /// body, fields and URL, in text and in (escaped) HTML.
    #[must_use]
    pub fn from_message(message: &Message) -> Self {
        let mut text = String::new();
        let mut html = format!("<h2>{}</h2>", escape_html(&message.title));
        if let Some(body) = message.body.as_deref().filter(|b| !b.is_empty()) {
            text.push_str(body);
            text.push('\n');
            html.push_str(&format!("<p>{}</p>", escape_html(body)));
        }
        if !message.fields.is_empty() {
            html.push_str("<ul>");
            for field in &message.fields {
                text.push_str(&format!("{}: {}\n", field.name, field.value));
                html.push_str(&format!(
                    "<li><strong>{}:</strong> {}</li>",
                    escape_html(&field.name),
                    escape_html(&field.value)
                ));
            }
            html.push_str("</ul>");
        }
        if let Some(url) = &message.url {
            text.push_str(url);
            text.push('\n');
            let url = escape_html(url);
            html.push_str(&format!("<p><a href=\"{url}\">{url}</a></p>"));
        }
        Self {
            subject: message.title.clone(),
            text,
            html,
        }
    }

    /// Renders a mailer template directory (`subject.t`, `text.t`,
    /// `html.t`), the same layout `cargo loco generate mailer` creates, with
    /// `locals` — typically the notifier itself.
    ///
    /// # Errors
    /// When a template is missing or fails to render.
    pub fn from_template(dir: &Dir<'_>, locals: &impl Serialize) -> Result<Self> {
        let content = Template::new(dir)?.render(&serde_json::to_value(locals)?)?;
        Ok(Self {
            subject: content.subject,
            text: content.text,
            html: content.html,
        })
    }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) async fn send(
    ctx: &AppContext,
    to: Vec<String>,
    from: Option<String>,
    content: EmailContent,
) -> Result<()> {
    let config = ctx
        .config
        .notifications
        .as_ref()
        .and_then(|n| n.email.as_ref());
    let mut to: Vec<String> = to.into_iter().filter(|t| !t.trim().is_empty()).collect();
    if to.is_empty() {
        to = config.map(|c| c.to.clone()).unwrap_or_default();
    }
    if to.is_empty() {
        return Err(Error::string(
            "email notification has no recipients: set notifications.email.to",
        ));
    }
    let from = from
        .or_else(|| config.and_then(|c| c.from.clone()))
        .unwrap_or_else(|| DEFAULT_FROM_SENDER.to_string());
    let Some(mailer) = &ctx.mailer else {
        return Err(Error::string(
            "email notification but no mailer is configured: add a `mailer:` config block",
        ));
    };
    mailer
        .mail_multi(&MultiEmail {
            from: Some(from),
            to,
            subject: content.subject,
            text: content.text,
            html: content.html,
            ..Default::default()
        })
        .await
}
