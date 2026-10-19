//! Provider presets, the code-for-token exchange and profile normalization.

use std::{collections::BTreeMap, sync::OnceLock, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{OAuthProviderConfig, OAuthStrategy, OAuthTokenAuth};

/// A user as an OAuth provider describes them, normalized across providers.
///
/// `uid` is the provider's stable account id — key your identities table by
/// `(provider, uid)`, never by `email` or `login`, which users can change.
/// `email` is set only when the provider reported one; trust it for account
/// linking only when `email_verified` is `Some(true)`.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    /// The provider's configured name (`github`, `google`, …).
    pub provider: String,
    /// The provider's stable id for the account.
    pub uid: String,
    /// Handle/username, when the provider has one (GitHub `login`).
    pub login: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Email address, when the provider returned one.
    pub email: Option<String>,
    /// Whether the provider vouches for `email`. `None` when it does not say.
    pub email_verified: Option<bool>,
    /// Avatar image URL.
    pub avatar_url: Option<String>,
    /// The access token, for calling the provider's API on the user's behalf.
    pub access_token: String,
    /// The refresh token, when the provider issued one.
    pub refresh_token: Option<String>,
    /// When `access_token` expires, when the provider said.
    pub expires_at: Option<DateTime<Utc>>,
    /// The scopes the provider granted, when it said (may differ from the
    /// ones requested).
    pub scope: Option<String>,
    /// The provider's profile response, unmodified.
    pub raw: Value,
}

/// Tokens stay out of logs.
impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("provider", &self.provider)
            .field("uid", &self.uid)
            .field("login", &self.login)
            .field("name", &self.name)
            .field("email", &self.email)
            .field("email_verified", &self.email_verified)
            .field("avatar_url", &self.avatar_url)
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_at", &self.expires_at)
            .field("scope", &self.scope)
            .field("raw", &self.raw)
            .finish()
    }
}

/// A provider's configuration with its preset filled in.
#[derive(Clone, PartialEq, Eq)]
pub struct Provider {
    /// The configured name.
    pub name: String,
    /// The preset in use.
    pub strategy: OAuthStrategy,
    /// OAuth client id.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
    /// Scopes requested.
    pub scopes: Vec<String>,
    /// Authorization endpoint.
    pub authorize_url: String,
    /// Token endpoint.
    pub token_url: String,
    /// Profile endpoint.
    pub userinfo_url: String,
    /// GitHub's emails endpoint.
    pub emails_url: Option<String>,
    /// Configured callback URL, if overridden.
    pub redirect_uri: Option<String>,
    /// Extra authorization query parameters.
    pub authorize_params: BTreeMap<String, String>,
    /// Whether to send a PKCE challenge.
    pub pkce: bool,
    /// Token endpoint client authentication.
    pub token_auth: OAuthTokenAuth,
}

/// The client secret stays out of logs.
impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("name", &self.name)
            .field("strategy", &self.strategy)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("scopes", &self.scopes)
            .field("authorize_url", &self.authorize_url)
            .field("token_url", &self.token_url)
            .field("userinfo_url", &self.userinfo_url)
            .field("emails_url", &self.emails_url)
            .field("redirect_uri", &self.redirect_uri)
            .field("authorize_params", &self.authorize_params)
            .field("pkce", &self.pkce)
            .field("token_auth", &self.token_auth)
            .finish()
    }
}

fn trim(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

impl Provider {
    /// Resolves `config` for the provider called `name`.
    ///
    /// # Errors
    ///
    /// When the strategy cannot be inferred from `name`, or a generic provider
    /// lacks an endpoint.
    pub fn resolve(name: &str, config: &OAuthProviderConfig) -> Result<Self, String> {
        let strategy = match config.strategy {
            Some(s) => s,
            None => match name {
                "github" => OAuthStrategy::Github,
                "google" => OAuthStrategy::Google,
                _ => {
                    return Err(format!(
                        "auth.oauth.providers.{name}: set `strategy` (github, google or \
                         generic)"
                    ))
                }
            },
        };
        let pick = |field: &Option<String>, default: String| {
            field.as_deref().map_or(default, str::to_string)
        };
        let (scopes, authorize_url, token_url, userinfo_url, emails_url) = match strategy {
            OAuthStrategy::Github => {
                let base = config
                    .base_url
                    .as_deref()
                    .map_or_else(|| "https://github.com".to_string(), trim);
                let api = config.api_url.as_deref().map_or_else(
                    || {
                        if config.base_url.is_some() {
                            format!("{base}/api/v3")
                        } else {
                            "https://api.github.com".to_string()
                        }
                    },
                    trim,
                );
                (
                    vec!["read:user".to_string(), "user:email".to_string()],
                    pick(
                        &config.authorize_url,
                        format!("{base}/login/oauth/authorize"),
                    ),
                    pick(
                        &config.token_url,
                        format!("{base}/login/oauth/access_token"),
                    ),
                    pick(&config.userinfo_url, format!("{api}/user")),
                    Some(pick(&config.emails_url, format!("{api}/user/emails"))),
                )
            }
            OAuthStrategy::Google => (
                oidc_scopes(),
                pick(
                    &config.authorize_url,
                    "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                ),
                pick(
                    &config.token_url,
                    "https://oauth2.googleapis.com/token".to_string(),
                ),
                pick(
                    &config.userinfo_url,
                    "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
                ),
                None,
            ),
            OAuthStrategy::Generic => {
                let need = |field: &Option<String>, key: &str| {
                    field.clone().ok_or_else(|| {
                        format!("auth.oauth.providers.{name}: generic providers need `{key}`")
                    })
                };
                (
                    oidc_scopes(),
                    need(&config.authorize_url, "authorize_url")?,
                    need(&config.token_url, "token_url")?,
                    need(&config.userinfo_url, "userinfo_url")?,
                    None,
                )
            }
        };
        Ok(Self {
            name: name.to_string(),
            strategy,
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            scopes: config.scopes.clone().unwrap_or(scopes),
            authorize_url,
            token_url,
            userinfo_url,
            emails_url,
            redirect_uri: config.redirect_uri.clone(),
            authorize_params: config.authorize_params.clone(),
            pkce: config.pkce.unwrap_or(true),
            token_auth: config.token_auth.unwrap_or_default(),
        })
    }

    /// The URL to send the browser to.
    ///
    /// # Errors
    ///
    /// When `authorize_url` is not a URL.
    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: Option<&str>,
    ) -> Result<String, String> {
        let mut url = url::Url::parse(&self.authorize_url)
            .map_err(|e| format!("{}: authorize_url: {e}", self.name))?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &self.client_id)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("state", state);
            if !self.scopes.is_empty() {
                query.append_pair("scope", &self.scopes.join(" "));
            }
            if let Some(challenge) = code_challenge {
                query
                    .append_pair("code_challenge", challenge)
                    .append_pair("code_challenge_method", "S256");
            }
            for (k, v) in &self.authorize_params {
                query.append_pair(k, v);
            }
        }
        Ok(url.into())
    }
}

fn oidc_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("loco-rs/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default()
    })
}

/// The token endpoint's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    /// Access token.
    pub access_token: String,
    /// Refresh token.
    pub refresh_token: Option<String>,
    /// Lifetime in seconds.
    pub expires_in: Option<i64>,
    /// Granted scopes.
    pub scope: Option<String>,
}

/// Reads a token response body. GitHub answers form-encoded unless asked for
/// JSON, and reports errors with a `200`, so both shapes and an `error` field
/// are handled whatever the status.
///
/// # Errors
///
/// When the body is an error or carries no access token.
pub fn parse_token_response(body: &str) -> Result<Tokens, String> {
    let value: Value = serde_json::from_str(body).unwrap_or_else(|_| {
        Value::Object(
            url::form_urlencoded::parse(body.as_bytes())
                .map(|(k, v)| (k.into_owned(), Value::String(v.into_owned())))
                .collect(),
        )
    });
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        let description = value
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(format!("{error} {description}").trim().to_string());
    }
    let text = |k: &str| value.get(k).and_then(Value::as_str).map(str::to_string);
    let access_token = text("access_token").ok_or("token response has no access_token")?;
    let expires_in = value.get("expires_in").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    });
    Ok(Tokens {
        access_token,
        refresh_token: text("refresh_token").filter(|s| !s.is_empty()),
        expires_in,
        scope: text("scope"),
    })
}

/// Exchanges an authorization `code` for tokens.
///
/// # Errors
///
/// On a transport error or an error response.
pub async fn exchange_code(
    provider: &Provider,
    code: &str,
    redirect_uri: &str,
    code_verifier: Option<&str>,
) -> Result<Tokens, String> {
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", provider.client_id.as_str()),
    ];
    if let Some(verifier) = code_verifier {
        form.push(("code_verifier", verifier));
    }
    let mut request = client()
        .post(&provider.token_url)
        .header(reqwest::header::ACCEPT, "application/json");
    match provider.token_auth {
        OAuthTokenAuth::Post => form.push(("client_secret", provider.client_secret.as_str())),
        OAuthTokenAuth::Basic => {
            request = request.basic_auth(&provider.client_id, Some(&provider.client_secret));
        }
    }
    let response = request
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token request failed: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("token response unreadable: {e}"))?;
    parse_token_response(&body).map_err(|e| format!("token endpoint answered {status}: {e}"))
}

async fn get_json(provider: &Provider, url: &str, token: &str) -> Result<Value, String> {
    let accept = match provider.strategy {
        OAuthStrategy::Github => "application/vnd.github+json",
        _ => "application/json",
    };
    let response = client()
        .get(url)
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, accept)
        .send()
        .await
        .map_err(|e| format!("GET {url} failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("GET {url} answered {status}"));
    }
    response
        .json()
        .await
        .map_err(|e| format!("GET {url}: invalid JSON: {e}"))
}

/// Fetches the profile for `tokens` and normalizes it.
///
/// # Errors
///
/// When the profile cannot be fetched or has no account id.
pub async fn fetch_identity(provider: &Provider, tokens: Tokens) -> Result<Identity, String> {
    let profile = get_json(provider, &provider.userinfo_url, &tokens.access_token).await?;
    let emails = match (&provider.emails_url, provider.strategy) {
        // Best effort: without the `user:email` scope this 404s, and the
        // public profile email (if any) is all there is.
        (Some(url), OAuthStrategy::Github) => get_json(provider, url, &tokens.access_token)
            .await
            .inspect_err(|e| tracing::debug!(error = %e, "github emails lookup failed"))
            .ok(),
        _ => None,
    };
    normalize(provider, profile, emails.as_ref(), tokens, Utc::now())
}

fn text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| match value.get(*k)? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn flag(value: &Value, key: &str) -> Option<bool> {
    match value.get(key)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Normalizes a profile (and, for GitHub, its `/user/emails` list) into an
/// [`Identity`].
///
/// # Errors
///
/// When the profile has no account id.
pub fn normalize(
    provider: &Provider,
    profile: Value,
    github_emails: Option<&Value>,
    tokens: Tokens,
    now: DateTime<Utc>,
) -> Result<Identity, String> {
    let (uid, login, name, mut email, mut email_verified, avatar_url) = match provider.strategy {
        OAuthStrategy::Github => (
            text(&profile, &["id"]),
            text(&profile, &["login"]),
            text(&profile, &["name"]),
            text(&profile, &["email"]),
            None,
            text(&profile, &["avatar_url"]),
        ),
        OAuthStrategy::Google => (
            text(&profile, &["sub"]),
            None,
            text(&profile, &["name"]),
            text(&profile, &["email"]),
            flag(&profile, "email_verified"),
            text(&profile, &["picture"]),
        ),
        OAuthStrategy::Generic => (
            text(&profile, &["sub", "id", "user_id"]),
            text(
                &profile,
                &["preferred_username", "login", "nickname", "username"],
            ),
            text(&profile, &["name", "display_name"]),
            text(&profile, &["email"]),
            flag(&profile, "email_verified"),
            text(&profile, &["picture", "avatar_url"]),
        ),
    };
    if let Some(primary) = github_emails
        .and_then(Value::as_array)
        .and_then(|list| {
            list.iter()
                .find(|e| flag(e, "primary") == Some(true) && flag(e, "verified") == Some(true))
        })
        .and_then(|e| text(e, &["email"]))
    {
        email = Some(primary);
        email_verified = Some(true);
    }
    let uid = uid.ok_or_else(|| format!("{}: profile has no account id", provider.name))?;
    Ok(Identity {
        provider: provider.name.clone(),
        uid,
        login,
        name,
        email,
        email_verified,
        avatar_url,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: tokens
            .expires_in
            .and_then(|s| now.checked_add_signed(chrono::Duration::seconds(s))),
        scope: tokens.scope,
        raw: profile,
    })
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use serde_json::json;

    use super::*;

    fn config(strategy: Option<OAuthStrategy>) -> OAuthProviderConfig {
        OAuthProviderConfig {
            strategy,
            client_id: "cid".to_string(),
            client_secret: "secret".to_string(),
            ..Default::default()
        }
    }

    fn tokens() -> Tokens {
        Tokens {
            access_token: "gho_token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_in: Some(3600),
            scope: Some("read:user,user:email".to_string()),
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn presets_resolve() {
        assert_debug_snapshot!(Provider::resolve("github", &config(None)).unwrap());
        assert_debug_snapshot!(Provider::resolve("google", &config(None)).unwrap());
    }

    #[test]
    fn github_enterprise_derives_both_hosts() {
        let mut c = config(None);
        c.base_url = Some("https://ghe.example.com/".to_string());
        let p = Provider::resolve("github", &c).unwrap();
        assert_eq!(
            p.authorize_url,
            "https://ghe.example.com/login/oauth/authorize"
        );
        assert_eq!(p.userinfo_url, "https://ghe.example.com/api/v3/user");
        assert_eq!(
            p.emails_url.as_deref(),
            Some("https://ghe.example.com/api/v3/user/emails")
        );
    }

    #[test]
    fn every_url_is_overridable() {
        let mut c = config(None);
        c.authorize_url = Some("http://127.0.0.1:1/a".to_string());
        c.token_url = Some("http://127.0.0.1:1/t".to_string());
        c.userinfo_url = Some("http://127.0.0.1:1/u".to_string());
        c.emails_url = Some("http://127.0.0.1:1/e".to_string());
        c.scopes = Some(vec!["repo".to_string()]);
        let p = Provider::resolve("github", &c).unwrap();
        assert_eq!(
            (
                p.authorize_url.as_str(),
                p.token_url.as_str(),
                p.userinfo_url.as_str(),
                p.emails_url.as_deref()
            ),
            (
                "http://127.0.0.1:1/a",
                "http://127.0.0.1:1/t",
                "http://127.0.0.1:1/u",
                Some("http://127.0.0.1:1/e")
            )
        );
        assert_eq!(p.scopes, vec!["repo"]);
    }

    #[test]
    fn unknown_names_need_a_strategy_and_generic_needs_urls() {
        assert!(Provider::resolve("okta", &config(None))
            .unwrap_err()
            .contains("strategy"));
        assert!(
            Provider::resolve("okta", &config(Some(OAuthStrategy::Generic)))
                .unwrap_err()
                .contains("authorize_url")
        );
    }

    #[test]
    fn authorize_url_carries_state_pkce_and_extra_params() {
        let mut c = config(None);
        c.authorize_params
            .insert("allow_signup".to_string(), "false".to_string());
        let p = Provider::resolve("github", &c).unwrap();
        assert_eq!(
            p.authorize_url(
                "http://localhost:5150/auth/github/callback",
                "st",
                Some("ch")
            )
            .unwrap(),
            "https://github.com/login/oauth/authorize?response_type=code&client_id=cid&\
             redirect_uri=http%3A%2F%2Flocalhost%3A5150%2Fauth%2Fgithub%2Fcallback&state=st&\
             scope=read%3Auser+user%3Aemail&code_challenge=ch&code_challenge_method=S256&\
             allow_signup=false"
        );
    }

    #[test]
    fn token_responses_in_both_shapes() {
        assert_eq!(
            parse_token_response("access_token=abc&scope=repo&token_type=bearer").unwrap(),
            Tokens {
                access_token: "abc".to_string(),
                refresh_token: None,
                expires_in: None,
                scope: Some("repo".to_string()),
            }
        );
        assert_eq!(
            parse_token_response(r#"{"access_token":"abc","refresh_token":"r","expires_in":"60"}"#)
                .unwrap()
                .expires_in,
            Some(60)
        );
        // GitHub reports a bad code with a 200.
        assert_eq!(
            parse_token_response(
                r#"{"error":"bad_verification_code","error_description":"The code passed is incorrect or expired."}"#
            )
            .unwrap_err(),
            "bad_verification_code The code passed is incorrect or expired."
        );
        assert!(parse_token_response("{}").is_err());
    }

    #[test]
    fn normalizes_github_preferring_the_primary_verified_email() {
        let p = Provider::resolve("github", &config(None)).unwrap();
        let profile = json!({
            "id": 583231, "login": "octocat", "name": "The Octocat",
            "email": "public@example.com",
            "avatar_url": "https://avatars.githubusercontent.com/u/583231"
        });
        let emails = json!([
            {"email": "old@example.com", "primary": false, "verified": true},
            {"email": "octo@example.com", "primary": true, "verified": true}
        ]);
        assert_debug_snapshot!(normalize(&p, profile, Some(&emails), tokens(), now()).unwrap());
    }

    #[test]
    fn an_unverified_primary_github_email_is_not_trusted() {
        let p = Provider::resolve("github", &config(None)).unwrap();
        let emails = json!([{"email": "x@example.com", "primary": true, "verified": false}]);
        let id = normalize(
            &p,
            json!({"id": 1, "login": "a"}),
            Some(&emails),
            tokens(),
            now(),
        )
        .unwrap();
        assert_eq!((id.email, id.email_verified), (None, None));
    }

    #[test]
    fn normalizes_google() {
        let p = Provider::resolve("google", &config(None)).unwrap();
        let profile = json!({
            "sub": "1100", "name": "Ada Lovelace", "email": "ada@example.com",
            "email_verified": true, "picture": "https://lh3.example/photo.jpg"
        });
        assert_debug_snapshot!(normalize(&p, profile, None, tokens(), now()).unwrap());
    }

    #[test]
    fn normalizes_generic_through_fallback_fields() {
        let mut c = config(Some(OAuthStrategy::Generic));
        c.authorize_url = Some("https://id.example/authorize".to_string());
        c.token_url = Some("https://id.example/token".to_string());
        c.userinfo_url = Some("https://id.example/userinfo".to_string());
        let p = Provider::resolve("acme", &c).unwrap();
        let id = normalize(
            &p,
            json!({"id": 7, "username": "grace", "avatar_url": "https://a/b.png",
                   "email": "g@example.com", "email_verified": "true"}),
            None,
            tokens(),
            now(),
        )
        .unwrap();
        assert_eq!(id.uid, "7");
        assert_eq!(id.login.as_deref(), Some("grace"));
        assert_eq!(id.avatar_url.as_deref(), Some("https://a/b.png"));
        assert_eq!(id.email_verified, Some(true));
        assert!(normalize(&p, json!({"name": "no id"}), None, tokens(), now()).is_err());
    }

    #[test]
    fn debug_output_redacts_tokens() {
        let p = Provider::resolve("google", &config(None)).unwrap();
        let id = normalize(&p, json!({"sub": "1"}), None, tokens(), now()).unwrap();
        let shown = format!("{id:?}");
        assert!(!shown.contains("gho_token") && !shown.contains("\"refresh\""));
    }
}
