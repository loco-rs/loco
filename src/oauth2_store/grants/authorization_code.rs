use std::{collections::HashMap, time::Instant};

use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    url,
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Response;
use subtle::ConstantTimeEq;

use crate::oauth2_store::error::{OAuth2ClientError, OAuth2ClientResult};

pub struct AuthorizationCodeCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
}

pub struct AuthorizationCodeUrlConfig {
    pub auth_url: String,
    pub token_url: Option<String>,
    pub redirect_url: String,
    pub profile_url: String,
    pub scopes: Vec<String>,
}

/// A struct that holds the OAuth2 client and the profile URL. - For
/// Authorization Code Grant
pub struct AuthorizationCodeClient {
    pub oauth2: BasicClient,
    pub profile_url: url::Url,
    pub http_client: reqwest::Client,
    pub flow_states: HashMap<String, (PkceCodeVerifier, Instant)>,
    pub scopes: Vec<Scope>,
    pub csrf_token_timeout: std::time::Duration,
}

impl AuthorizationCodeClient {
    /// Create a new instance of `OAuth2Client`.
    pub fn new(
        credentials: AuthorizationCodeCredentials,
        config: AuthorizationCodeUrlConfig,
        timeout_seconds: Option<u64>,
    ) -> OAuth2ClientResult<Self> {
        let client_id = ClientId::new(credentials.client_id);
        let client_secret = credentials.client_secret.map(ClientSecret::new);
        let auth_url = AuthUrl::new(config.auth_url)?;
        let token_url = if let Some(token_url) = config.token_url {
            Some(TokenUrl::new(token_url)?)
        } else {
            None
        };
        let redirect_url = RedirectUrl::new(config.redirect_url)?;
        let oauth2 = BasicClient::new(client_id, client_secret, auth_url, token_url)
            .set_redirect_uri(redirect_url);
        let profile_url = url::Url::parse(&config.profile_url)?;
        let scopes = config
            .scopes
            .iter()
            .map(|scope| Scope::new(scope.to_string()))
            .collect();
        Ok(Self {
            oauth2,
            profile_url,
            http_client: reqwest::Client::new(),
            flow_states: HashMap::new(),
            scopes,
            csrf_token_timeout: std::time::Duration::from_secs(timeout_seconds.unwrap_or(10 * 60)),
        })
    }
    fn remove_expire_flow(&mut self) {
        // Remove expired tokens
        self.flow_states
            .retain(|_, (_, created_at)| created_at.elapsed() < self.csrf_token_timeout);
    }
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    // Convert the strings to bytes for comparison.
    // Note: This assumes both slices are of the same length.
    // You might want to handle differing lengths explicitly, depending on your
    // security requirements.
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[async_trait::async_trait]
pub trait AuthorizationCodeGrantTrait: Send + Sync {
    fn get_authorization_code_client(&mut self) -> &mut AuthorizationCodeClient;
    /// Get authorization URL
    fn get_authorization_url(&mut self) -> (Url, CsrfToken) {
        let client = self.get_authorization_code_client();
        // Clear outdated flow states
        client.remove_expire_flow();

        // Generate a PKCE challenge.
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client.oauth2.authorize_url(CsrfToken::new_random);
        // Add scopes
        for scope in &client.scopes {
            auth_request = auth_request.add_scope(scope.clone());
        }
        // Generate the full authorization URL.
        let (auth_url, csrf_token) = auth_request
            // Set the PKCE code challenge.
            .set_pkce_challenge(pkce_challenge)
            .url();
        // Store the CSRF token, PKCE Verifier and the time it was created.
        let csrf_secret = csrf_token.secret().clone();
        client
            .flow_states
            .insert(csrf_secret.clone(), (pkce_verifier, Instant::now()));
        (auth_url, csrf_token)
    }

    async fn verify_user_with_code(
        &mut self,
        code: String,
        state: String,
        csrf_token: String,
    ) -> OAuth2ClientResult<(BasicTokenResponse, Response)> {
        let client = self.get_authorization_code_client();
        // Clear outdated flow states
        client.remove_expire_flow();
        // Compare csrf token, use subtle to prevent time attack
        if !constant_time_compare(&csrf_token, &state) {
            return Err(OAuth2ClientError::CsrfTokenError);
        }
        // Get the pkce_verifier for exchanging code
        let (pkce_verifier, _) = match client.flow_states.remove(&csrf_token) {
            None => {
                return Err(OAuth2ClientError::CsrfTokenError);
            }
            Some(item) => item,
        };
        // Exchange the code with a token
        let token = client
            .oauth2
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;
        let profile = client
            .http_client
            .get(client.profile_url.clone())
            .bearer_auth(token.access_token().secret().to_owned())
            .send()
            .await?;
        Ok((token, profile))
    }
}

impl AuthorizationCodeGrantTrait for AuthorizationCodeClient {
    fn get_authorization_code_client(&mut self) -> &mut AuthorizationCodeClient {
        self
    }
}

#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, with_settings};

    use super::*;

    const CLIENT_ID: &'static str = "client_id";
    const CLIENT_SECRET: &'static str = "client_secret";
    const AUTH_URL: &'static str = "https://oauth.com/auth_url";
    const TOKEN_URL: &'static str = "https://oauth.com/token_url";
    const REDIRECT_URL: &'static str = "https://oauth.com/redirect_url";
    const PROFILE_URL: &'static str = "https://oauth.com/profile_url";
    const SCOPE_1: &'static str = "https://oauth.com/scope_1";

    fn get_base_url(url: &Url) -> String {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or_default(); // Get the host as a str, default to empty string if not present
        format!("{}://{}", scheme, host)
    }
    fn get_base_url_with_path(url: &Url) -> String {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or_default(); // Get the host as a str, default to empty string if not present
        let path = url.path();
        format!("{}://{}{}", scheme, host, path)
    }
    fn create_client() -> OAuth2ClientResult<AuthorizationCodeClient> {
        let credentials = AuthorizationCodeCredentials {
            client_id: CLIENT_ID.to_string(),
            client_secret: Some(CLIENT_SECRET.to_string()),
        };
        let config = AuthorizationCodeUrlConfig {
            auth_url: AUTH_URL.to_string(),
            token_url: Some(TOKEN_URL.to_string()),
            redirect_url: REDIRECT_URL.to_string(),
            profile_url: PROFILE_URL.to_string(),
            scopes: vec![SCOPE_1.to_string()],
        };
        AuthorizationCodeClient::new(credentials, config, None)
    }

    #[tokio::test]
    async fn get_authorization_url() -> OAuth2ClientResult<()> {
        let mut client = create_client()?;
        let (url, csrf_token) = client.get_authorization_url();
        println!("url: {}, csrf: {:?}", url, csrf_token.secret());
        let base_url_with_path = get_base_url_with_path(&url);
        // compare between the auth_url with the base url
        assert_eq!(AUTH_URL.to_string(), base_url_with_path);
        let query_pairs = url.query_pairs();

        Ok(())
    }
}
