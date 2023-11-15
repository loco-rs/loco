//! This module defines an [`EmailSender`] responsible for sending emails using either the SMTP
//! protocol. It includes an asynchronous method `mail` for sending
//! emails with options like sender, recipient, subject, and content.

use super::{Email, Result, DEFAULT_FROM_SENDER};
use crate::{config, errors::Error};
use lettre::{
    message::MultiPart, transport::smtp::authentication::Credentials, AsyncTransport, Message,
    Tokio1Executor, Transport,
};

/// An enumeration representing the possible transport methods for sending emails.
#[derive(Clone)]
pub enum EmailTransport {
    /// SMTP (Simple Mail Transfer Protocol) transport.
    Smtp(lettre::AsyncSmtpTransport<lettre::Tokio1Executor>),
    /// Test/stub transport for testing purposes.
    Test(lettre::transport::stub::StubTransport),
}

/// A structure representing the email sender, encapsulating the chosen transport method.
#[derive(Clone)]
pub struct EmailSender {
    pub transport: EmailTransport,
}

impl EmailSender {
    /// Creates a new `EmailSender` using the SMTP transport method based on the provided SMTP configuration.
    ///
    /// # Errors
    ///
    /// when could not initialize SMTP transport
    pub fn smtp(config: &config::SmtpMailer) -> Result<Self> {
        let mut email_builder = if config.secure {
            lettre::AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|error| {
                    tracing::error!(error = error.to_string(), "error creating SMTP transport");
                    Error::Any(format!("error initialize smtp mailer").into())
                })?
                .port(config.port)
        } else {
            lettre::AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port)
        };

        if let Some(auth) = config.auth.as_ref() {
            email_builder = email_builder
                .credentials(Credentials::new(auth.user.clone(), auth.password.clone()));
        }

        Ok(Self {
            transport: EmailTransport::Smtp(email_builder.build()),
        })
    }

    /// Sends an email using the configured transport method.
    ///
    /// # Errors
    ///
    /// When email did't send successfully or has an error to build the message
    pub async fn mail(&self, email: &Email) -> Result<()> {
        let content = MultiPart::alternative_plain_html(email.text.clone(), email.html.clone());
        let mut builder = Message::builder()
            .from(
                email
                    .from
                    .clone()
                    .unwrap_or_else(|| DEFAULT_FROM_SENDER.to_string())
                    .parse()?,
            )
            .to(email.to.parse()?);

        if let Some(reply_to) = &email.reply_to {
            builder = builder.reply_to(reply_to.parse()?);
        }

        let msg = builder
            .subject(email.subject.clone())
            .multipart(content)
            .map_err(|error| {
                tracing::error!(error = error.to_string(), "error building email message");
                Error::Any(format!("error building email message").into())
            })?;

        match &self.transport {
            EmailTransport::Smtp(xp) => {
                xp.send(msg).await?;
            }
            EmailTransport::Test(xp) => {
                xp.send(&msg)
                    .map_err(|_| Error::Any(format!("sending email error").into()))?;
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use insta::{assert_debug_snapshot, with_settings};
    use lettre::transport::stub::StubTransport;

    #[tokio::test]
    async fn can_send_email() {
        let stub = StubTransport::new_ok();

        let sender = EmailSender {
            transport: EmailTransport::Test(stub.clone()),
        };

        let html = r#"
;<html>
    <body>
        Test Message
    </body>
</html>"#;

        let data = Email {
            from: Some("test@framework.com".to_string()),
            to: "user1@framework.com".to_string(),
            reply_to: None,
            subject: "Email Subject".to_string(),
            text: "Welcome".to_string(),
            html: html.to_string(),
        };
        assert!(sender.mail(&data).await.is_ok());

        with_settings!({filters => vec![
            (r"[0-9A-Za-z]+{40}", "IDENTIFIER"),
            (r"\w+, \d{1,2} \w+ \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}", "DATE")
        ]}, {
            assert_debug_snapshot!(stub.messages());
        });
    }
}
