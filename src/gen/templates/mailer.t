{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/{{module_name}}.rs"
skip_exists: true
message: "A mailer `{{struct_name}}` was added successfully."
injections:
- into: "src/mailers/mod.rs"
  append: true
  content: "pub mod {{ module_name }};"
---
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

static welcome: Dir<'_> = include_dir!("src/mailers/{{module_name}}/welcome");

#[allow(clippy::module_name_repetitions)]
pub struct {{struct_name}} {}
impl Mailer for {{struct_name}} {}
impl {{struct_name}} {
    pub async fn send_welcome(ctx: &AppContext, to: &str, msg: &str) -> Result<()> {
        Self::mail_template(
            ctx,
            &welcome,
            mailer::Args {
                to,
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
