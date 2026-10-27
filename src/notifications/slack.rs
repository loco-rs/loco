//! Slack incoming webhooks.
//!
//! The default payload is a single `text` message in Slack's `mrkdwn`: the
//! title in bold, then the body, the fields as `*name:* value` lines and the
//! URL as a link. Override [`super::Notifier::slack_payload`] to send Block Kit
//! `blocks` instead.

use serde_json::{json, Value};

use super::{http_error, Message};
use crate::Result;

/// Escapes the three characters Slack's `mrkdwn` treats as control
/// characters, so user-supplied text cannot inject links or mentions.
#[must_use]
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The default Slack payload for `message`.
#[must_use]
pub fn payload(message: &Message) -> Value {
    let mut text = format!("*{}*", escape(&message.title));
    if let Some(body) = message.body.as_deref().filter(|b| !b.is_empty()) {
        text.push('\n');
        text.push_str(&escape(body));
    }
    for field in &message.fields {
        text.push_str(&format!(
            "\n*{}:* {}",
            escape(&field.name),
            escape(&field.value)
        ));
    }
    if let Some(url) = &message.url {
        // Inside `<url|label>` only `|` and `>` are structural.
        let url = url.replace('>', "%3E").replace('|', "%7C");
        text.push_str(&format!("\n<{url}|Open>"));
    }
    json!({ "text": text })
}

pub(super) async fn send(http: &reqwest::Client, url: &str, payload: &Value) -> Result<()> {
    let response = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| http_error("slack", e))?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(crate::Error::Message(format!(
        "slack webhook answered {status}: {}",
        body.chars().take(200).collect::<String>()
    )))
}
