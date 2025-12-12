//! This module defines an [`EmailSender`] responsible for sending emails using
//! either the SMTP protocol. It includes an asynchronous method `mail` for
//! sending emails with options like sender, recipient, subject, and content.

use lettre::{
    message::{header, MultiPart},
    transport::smtp::{authentication::Credentials, extension::ClientId},
    AsyncTransport, Message, Tokio1Executor, Transport,
};
use tracing::error;

use super::{Email, Result, DEFAULT_FROM_SENDER};
use crate::{config, errors::Error};

/// An enumeration representing the possible transport methods for sending
/// emails.
#[derive(Clone, Debug)]
pub enum EmailTransport {
    /// SMTP (Simple Mail Transfer Protocol) transport.
    Smtp(lettre::AsyncSmtpTransport<lettre::Tokio1Executor>),
    /// Test/stub transport for testing purposes.
    Test(lettre::transport::stub::StubTransport),
}

/// A structure representing the email sender, encapsulating the chosen
/// transport method.
#[derive(Clone, Debug)]
pub struct EmailSender {
    pub transport: EmailTransport,
}

#[cfg(feature = "testing")]
#[derive(Default, Debug)]
pub struct Deliveries {
    pub count: usize,
    pub messages: Vec<String>,
}

impl EmailSender {
    /// Creates a new `EmailSender` using the SMTP transport method based on the
    /// provided SMTP configuration.
    ///
    /// # Errors
    ///
    /// when could not initialize SMTP transport
    pub fn smtp(config: &config::SmtpMailer) -> Result<Self> {
        let mut email_builder = if config.secure {
            lettre::AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|error| {
                    error!(err.msg = %error, err.detail = ?error, "smtp_init_error");
                    error
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

        if let Some(hello_name) = config.hello_name.as_ref() {
            email_builder = email_builder.hello_name(ClientId::Domain(hello_name.clone()));
        }

        Ok(Self {
            transport: EmailTransport::Smtp(email_builder.build()),
        })
    }

    #[must_use]
    pub fn stub() -> Self {
        Self {
            transport: EmailTransport::Test(lettre::transport::stub::StubTransport::new_ok()),
        }
    }

    #[cfg(feature = "testing")]
    #[must_use]
    pub fn deliveries(&self) -> Deliveries {
        if let EmailTransport::Test(stub) = &self.transport {
            return Deliveries {
                count: stub.messages().len(),
                messages: stub
                    .messages()
                    .iter()
                    .map(|(_, content)| content.clone())
                    .collect(),
            };
        }

        Deliveries::default()
    }

    /// Sends an email using the configured transport method.
    ///
    /// # Errors
    ///
    /// When email doesn't send successfully or has an error to build the
    /// message
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

        if let Some(bcc) = &email.bcc {
            builder = builder.bcc(bcc.parse()?);
        }

        if let Some(cc) = &email.cc {
            builder = builder.cc(cc.parse()?);
        }

        if let Some(reply_to) = &email.reply_to {
            builder = builder.reply_to(reply_to.parse()?);
        }

        if let Some(headers) = &email.headers {
            if let Some(references) = &headers.references {
                builder = builder.header(header::References::from(references.clone()));
            }
            if let Some(in_reply_to) = &headers.in_reply_to {
                builder = builder.header(header::InReplyTo::from(in_reply_to.clone()));
            }
            if let Some(message_id) = &headers.message_id {
                builder = builder.header(header::MessageId::from(message_id.clone()));
            }
        }

        let msg = builder
            .subject(email.subject.clone())
            .multipart(content)
            .map_err(|error| {
                error!(err.msg = %error, err.detail = ?error, "email_building_error");
                error
            })?;

        match &self.transport {
            EmailTransport::Smtp(xp) => {
                xp.send(msg).await?;
            }
            EmailTransport::Test(xp) => {
                xp.send(&msg)
                    .map_err(|e| Error::Message(format!("sending email error: {e}")))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use insta::{assert_debug_snapshot, with_settings};
    use lettre::transport::stub::StubTransport;

    use super::*;

    #[tokio::test]
    async fn can_send_email() {
        let stub = StubTransport::new_ok();

        let sender = EmailSender {
            transport: EmailTransport::Test(stub.clone()),
        };

        let html = r"
;<html>
    <body>
        Test Message
    </body>
</html>";

        let data = Email {
            from: Some("test@framework.com".to_string()),
            to: "user1@framework.com".to_string(),
            reply_to: None,
            subject: "Email Subject".to_string(),
            text: "Welcome".to_string(),
            html: html.to_string(),
            bcc: None,
            cc: None,
            headers: None,
        };
        assert!(sender.mail(&data).await.is_ok());

        with_settings!({filters => vec![
            (r"[0-9A-Za-z]+{40}", "IDENTIFIER"),
            (r"\w+, \d{1,2} \w+ \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}", "DATE")
        ]}, {
            assert_debug_snapshot!(stub.messages());
        });
    }

    #[tokio::test]
    async fn can_send_email_with_custom_headers() {
        let stub = StubTransport::new_ok();

        let sender = EmailSender {
            transport: EmailTransport::Test(stub.clone()),
        };

        let html = r"
<html>
    <body>
        Test Message with Headers
    </body>
</html>";

        let headers = crate::mailer::EmailHeaders {
            references: Some("<notification-item-123@example.com>".to_string()),
            in_reply_to: Some("<notification-item-123@example.com>".to_string()),
            message_id: Some("<notification-item-123-1234567890@example.com>".to_string()),
        };

        let data = Email {
            from: Some("test@framework.com".to_string()),
            to: "user1@framework.com".to_string(),
            reply_to: None,
            subject: "Email Subject with Headers".to_string(),
            text: "Welcome with headers".to_string(),
            html: html.to_string(),
            bcc: None,
            cc: None,
            headers: Some(headers),
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
