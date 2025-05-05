//! This module defines the email-related functionality, including the `Mailer`
//! trait and its implementation, `Email` structure, and the `MailerWorker` for
//! asynchronous email processing.

mod email_sender;
mod template;

use self::template::Template;
use super::{app::AppContext, Result};
use crate::prelude::BackgroundWorker;
use async_trait::async_trait;
pub use email_sender::EmailSender;
use include_dir::Dir;
use serde::{Deserialize, Serialize};
use tracing::error;

pub const DEFAULT_FROM_SENDER: &str = "System <system@example.com>";

/// The arguments struct for specifying email details such as sender, recipient,
/// reply-to, and locals.
#[derive(Debug, Clone, Default)]
pub struct Args {
    pub from: Option<String>,
    pub to: String,
    pub reply_to: Option<String>,
    pub locals: serde_json::Value,
    pub bcc: Option<String>,
    pub cc: Option<String>,
}

/// The structure representing an email details.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Email {
    /// Mailbox to `From` header
    pub from: Option<String>,
    /// Mailbox to `To` header
    pub to: String,
    /// Mailbox to `ReplyTo` header
    pub reply_to: Option<String>,
    /// Subject header to message
    pub subject: String,
    /// Plain text message
    pub text: String,
    /// HTML template
    pub html: String,
    /// BCC header to message
    pub bcc: Option<String>,
    /// CC header to message
    pub cc: Option<String>,
}

/// The options struct for configuring the email sender.
#[derive(Default, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct MailerOpts {
    pub from: String,
    pub reply_to: Option<String>,
}

/// The `Mailer` trait defines methods for sending emails and processing email
/// templates.
#[async_trait]
pub trait Mailer {
    /// Returns default options for the mailer.
    #[must_use]
    fn opts() -> MailerOpts {
        MailerOpts {
            from: DEFAULT_FROM_SENDER.to_string(),
            ..Default::default()
        }
    }

    /// Sends an email using the provided [`AppContext`] and email details.
    async fn mail(ctx: &AppContext, email: &Email) -> Result<()> {
        let opts = Self::opts();
        let mut email = email.clone();

        email.from = Some(email.from.unwrap_or_else(|| opts.from.clone()));
        email.reply_to = email.reply_to.or_else(|| opts.reply_to.clone());

        MailerWorker::perform_later(ctx, email.clone()).await?;
        Ok(())
    }

    /// Renders and sends an email using the provided [`AppContext`], template
    /// directory, and arguments.
    async fn mail_template(ctx: &AppContext, dir: &Dir<'_>, args: Args) -> Result<()> {
        let content = Template::new(dir).render(&args.locals)?;
        Self::mail(
            ctx,
            &Email {
                from: args.from.clone(),
                to: args.to.clone(),
                reply_to: args.reply_to.clone(),
                subject: content.subject,
                text: content.text,
                html: content.html,
                bcc: args.bcc.clone(),
                cc: args.cc.clone(),
            },
        )
        .await
    }
}

/// The [`MailerWorker`] struct represents a worker responsible for asynchronous
/// email processing.
#[allow(clippy::module_name_repetitions)]
pub struct MailerWorker {
    pub ctx: AppContext,
}

impl MailerWorker {
    pub fn build(ctx: &AppContext) -> MailerWorker {
        Self { ctx: ctx.clone() }
    }
}

/// Implementation of the [`Worker`] trait for the [`MailerWorker`].
#[async_trait]
impl BackgroundWorker<Email> for MailerWorker {
    fn queue() -> Option<String> {
        Some("mailer".to_string())
    }

    /// Performs the email sending operation using the provided [`AppContext`]
    /// and email details.
    async fn perform(&self, email: Email) -> crate::Result<()> {
        if let Some(mailer) = &self.ctx.mailer {
            let res = mailer.mail(&email).await;
            match res {
                Ok(res) => Ok(res),
                Err(err) => {
                    error!(err = err.to_string(), "mailer error");
                    Err(err)
                }
            }
        } else {
            let err = crate::Error::Message(
                "attempting to send email but no email sender configured".to_string(),
            );
            error!(err = err.to_string(), "mailer error");
            Err(err)
        }
    }
}
