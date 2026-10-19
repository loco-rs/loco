#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::users;

static notice: Dir<'_> = include_dir!("src/mailers/deactivation/notice");

/// Account-lifecycle email.
///
/// `impl Mailer` is an empty trait whose purpose is to give this type
/// `mail_template`. The transport comes from `ctx` and is configured under
/// `mailer:` in `config/<env>.yaml` — so there is no SMTP client to write and
/// no `std::env::var` to read.
pub struct DeactivationMailer {}
impl Mailer for DeactivationMailer {}

impl DeactivationMailer {
    /// Tell a user their account was deactivated.
    ///
    /// # Errors
    ///
    /// When sending fails.
    pub async fn send_notice(
        ctx: &AppContext,
        user: &users::Model,
        retention_days: i64,
    ) -> Result<()> {
        Self::mail_template(
            ctx,
            &notice,
            mailer::Args {
                to: user.email.clone(),
                locals: json!({
                    "name": user.name,
                    "retentionDays": retention_days,
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
