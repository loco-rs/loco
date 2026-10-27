{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/notifiers/{{module_name}}.rs"
skip_exists: true
message: "A notifier `{{struct_name}}` was added successfully. Send it with `{{struct_name}} { .. }.deliver_later(&ctx).await`."
injections:
- into: "src/notifiers/mod.rs"
  append: true
  content: "pub mod {{ module_name }};"
- into: "src/lib.rs"
  append: true
  skip_if: "pub mod notifiers;"
  content: "pub mod notifiers;"
---
use loco_rs::notifications::{Channel, Message};
use loco_rs::prelude::*;
use serde::Serialize;

/// The data of one `{{struct_name}}` notification. It is also sent as `data`
/// in webhook payloads.
///
/// Send it from a controller, task or worker; delivery happens in the
/// background and never fails the caller:
///
/// ```rust,ignore
/// {{struct_name}} { text: "...".to_string() }.deliver_later(&ctx).await;
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct {{struct_name}} {
    pub text: String,
}

impl Notifier for {{struct_name}} {
    fn message(&self) -> Message {
        Message::new("{{ module_name | title_case }}").body(&self.text)
    }

    /// Where it goes. A channel with no destination in `config/*.yaml`
    /// (`notifications:`) is skipped with a warning.
    ///
    /// Other channels: `Channel::slack_url(url)`, `Channel::webhook()`,
    /// `Channel::webhook_url(url).signed(secret)`, `Channel::email()` and
    /// `Channel::email_to(address)` (needs the `mailer` feature).
    fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
        vec![Channel::slack()]
    }
}
