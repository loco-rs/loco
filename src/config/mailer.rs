use serde::{Deserialize, Serialize};

/// Mailer configuration
///
/// Example (development), to capture mails with something like [mailcrab](https://github.com/tweedegolf/mailcrab):
/// ```yaml
/// # config/development.yaml
/// mailer:
///   smtp:
///     enable: true
///     host: localhost
///     port: 1025
///     secure: false
/// ```
///
/// Example (production) with implicit TLS on port 465 (SMTPS) — note `tls: implicit`,
/// since `secure: true` only does STARTTLS (port 587):
/// ```yaml
/// mailer:
///   smtp:
///     enable: true
///     host: smtp.example.com
///     port: 465
///     tls: implicit
///     auth:
///       user: postmaster@mg.example.com
///       password: "{{ get_env(name='SMTP_PASSWORD') }}"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mailer {
    pub smtp: Option<SmtpMailer>,

    #[serde(default)]
    pub stub: bool,
}

/// TLS mode for the SMTP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MailerTls {
    /// Opportunistic TLS: connect in cleartext, then upgrade with `STARTTLS`
    /// (the SMTP submission port, 587). This is what `secure: true` selects.
    Starttls,
    /// Implicit TLS: the connection is encrypted from the first byte (SMTPS,
    /// port 465). Required by providers that listen for implicit TLS on 465 —
    /// `STARTTLS` does not work there.
    Implicit,
    /// No transport security (cleartext). For local sinks like Mailpit only.
    None,
}

/// SMTP mailer configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpMailer {
    pub enable: bool,
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port/
    pub port: u16,
    /// Enable TLS.
    ///
    /// Backwards-compatible shorthand: `true` selects `STARTTLS`, `false`
    /// cleartext. Prefer [`SmtpMailer::tls`] for implicit TLS (port 465);
    /// when `tls` is set it takes precedence over this flag.
    pub secure: bool,
    /// Explicit TLS mode. When set, overrides [`SmtpMailer::secure`]. Use
    /// [`MailerTls::Implicit`] for SMTPS on port 465.
    #[serde(default)]
    pub tls: Option<MailerTls>,
    /// Auth SMTP server
    pub auth: Option<MailerAuth>,
    /// Optional EHLO client ID instead of hostname
    pub hello_name: Option<String>,
}

impl SmtpMailer {
    /// Resolve the effective TLS mode: explicit [`SmtpMailer::tls`] when set,
    /// otherwise derived from the legacy [`SmtpMailer::secure`] flag.
    #[must_use]
    pub fn tls_mode(&self) -> MailerTls {
        self.tls.unwrap_or(if self.secure {
            MailerTls::Starttls
        } else {
            MailerTls::None
        })
    }
}

/// Authentication details for the mailer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuth {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}
