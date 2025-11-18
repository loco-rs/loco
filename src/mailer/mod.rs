//! This module defines the email-related functionality, including the `Mailer`
//! trait and its implementation, `Email` structure, and the `MailerWorker` for
//! asynchronous email processing.

mod email_sender;
mod template;

use async_trait::async_trait;
pub use email_sender::EmailSender;
use include_dir::Dir;
use serde::{Deserialize, Serialize};
use tracing::error;

use self::template::Template;
use super::{app::AppContext, Result};
use crate::prelude::BackgroundWorker;

pub const DEFAULT_FROM_SENDER: &str = "System <system@example.com>";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailHeaders {
    pub references: Option<String>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
}

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
    pub headers: Option<EmailHeaders>,
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
    /// Custom headers for the email (e.g., References, In-Reply-To, Message-ID)
    pub headers: Option<EmailHeaders>,
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
        Self::mail_template_with_shared(ctx, dir, &[], args).await
    }

    /// Renders and sends an email using the provided [`AppContext`], template
    /// directory, shared template directories, and arguments.
    ///
    /// This allows multiple mailers to share common templates (e.g., a base HTML layout).
    /// Templates from shared directories are loaded first, then templates from the main
    /// directory. This means templates in the main directory can extend templates from
    /// shared directories.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// use include_dir::{include_dir, Dir};
    /// use loco_rs::prelude::*;
    ///
    /// // Shared base template directory
    /// static shared_base: Dir<'_> = include_dir!("src/mailers/shared");
    ///
    /// // Welcome mailer templates
    /// static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
    ///
    /// // Send email with shared templates
    /// Self::mail_template_with_shared(
    ///     ctx,
    ///     &welcome,
    ///     &[&shared_base],
    ///     mailer::Args {
    ///         to: "user@example.com".to_string(),
    ///         locals: json!({"name": "User"}),
    ///         ..Default::default()
    ///     },
    /// )
    /// .await?;
    /// ```
    async fn mail_template_with_shared(
        ctx: &AppContext,
        dir: &Dir<'_>,
        shared_dirs: &[&Dir<'_>],
        args: Args,
    ) -> Result<()> {
        let content = Template::new_with_shared(dir, shared_dirs)?.render(&args.locals)?;
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
                headers: args.headers.clone(),
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

/// Implementation of the [`Worker`] trait for the [`MailerWorker`].
#[async_trait]
impl BackgroundWorker<Email> for MailerWorker {
    fn queue() -> Option<String> {
        Some("mailer".to_string())
    }

    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
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
