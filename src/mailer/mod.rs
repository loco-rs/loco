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

/// Default background-queue priority used when enqueuing mailer jobs. Higher
/// values are processed sooner (see [`crate::bgworker::Queue::enqueue`]).
pub const DEFAULT_MAILER_PRIORITY: i32 = 100;

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
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct MailerOpts {
    pub from: String,
    pub reply_to: Option<String>,
    /// Background-queue priority for enqueued mailer jobs.
    pub priority: i32,
}

impl Default for MailerOpts {
    fn default() -> Self {
        Self {
            from: DEFAULT_FROM_SENDER.to_string(),
            reply_to: None,
            priority: DEFAULT_MAILER_PRIORITY,
        }
    }
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

        MailerWorker::perform_later_with_priority(ctx, email.clone(), Some(opts.priority)).await?;
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
    /// This lets multiple mailers share common templates (e.g. a base HTML
    /// layout). Templates from `shared_dirs` are loaded first, then templates
    /// from the main directory, so main-directory templates can extend shared
    /// ones and override any with the same name.
    ///
    /// # Errors
    /// Returns an error if a template is missing/invalid or the send fails.
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

    /// Sends an email **synchronously**, bypassing the background worker queue
    /// (Rails' `deliver_now`). Prefer [`Mailer::mail`] (which enqueues, like
    /// `deliver_later`) unless you specifically need the send to complete inline.
    ///
    /// # Errors
    /// Returns an error if no mailer is configured or the send fails.
    async fn deliver_now(ctx: &AppContext, email: &Email) -> Result<()> {
        let opts = Self::opts();
        let mut email = email.clone();
        email.from = Some(email.from.unwrap_or_else(|| opts.from.clone()));
        email.reply_to = email.reply_to.or_else(|| opts.reply_to.clone());
        send_now(ctx, &email).await
    }

    /// Renders a template and sends it **synchronously** (see
    /// [`Mailer::deliver_now`]). The template-rendering counterpart to
    /// [`Mailer::mail_template`], which enqueues instead.
    ///
    /// # Errors
    /// Returns an error if rendering fails, no mailer is configured, or the send fails.
    async fn mail_template_now(ctx: &AppContext, dir: &Dir<'_>, args: Args) -> Result<()> {
        let content = Template::new(dir)?.render(&args.locals)?;
        Self::deliver_now(
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

/// Sends an already-prepared email synchronously through the context's
/// configured [`EmailSender`], bypassing the background queue. Errors if no
/// mailer is configured.
async fn send_now(ctx: &AppContext, email: &Email) -> Result<()> {
    if let Some(mailer) = &ctx.mailer {
        mailer.mail(email).await.inspect_err(|err| {
            error!(err = err.to_string(), "mailer error");
        })
    } else {
        let err = crate::Error::Message(
            "attempting to send email but no email sender configured".to_string(),
        );
        error!(err = err.to_string(), "mailer error");
        Err(err)
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
        send_now(&self.ctx, &email).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMailer;
    impl Mailer for TestMailer {}

    #[tokio::test]
    async fn deliver_now_sends_synchronously_without_worker() {
        let mut ctx = crate::tests_cfg::app::get_app_context().await;
        ctx.mailer = Some(EmailSender::stub());

        let email = Email {
            from: None,
            to: "user1@framework.com".to_string(),
            reply_to: None,
            subject: "Subject".to_string(),
            text: "Welcome".to_string(),
            html: "<html><body>Welcome</body></html>".to_string(),
            bcc: None,
            cc: None,
            headers: None,
        };

        TestMailer::deliver_now(&ctx, &email)
            .await
            .expect("deliver_now should succeed");

        let deliveries = ctx.mailer.as_ref().unwrap().deliveries();
        assert_eq!(deliveries.count, 1);
    }

    #[tokio::test]
    async fn deliver_now_errors_when_no_mailer_configured() {
        let ctx = crate::tests_cfg::app::get_app_context().await;
        assert!(ctx.mailer.is_none());

        let email = Email {
            from: None,
            to: "user1@framework.com".to_string(),
            reply_to: None,
            subject: "Subject".to_string(),
            text: "Welcome".to_string(),
            html: "<html><body>Welcome</body></html>".to_string(),
            bcc: None,
            cc: None,
            headers: None,
        };

        let err = TestMailer::deliver_now(&ctx, &email)
            .await
            .expect_err("deliver_now should error without a configured mailer");
        assert_eq!(
            err.to_string(),
            "attempting to send email but no email sender configured"
        );
    }
}
