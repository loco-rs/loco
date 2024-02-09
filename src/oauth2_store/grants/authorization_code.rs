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
            .await.map_err(|e|OAuth2ClientError::ProfileError(e))?;
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
    use oauth2::url::form_urlencoded;
    use serde::{Deserialize, Serialize};

    use super::*;

    struct Settings {
        client_id: String,
        client_secret: String,
        auth_url: String,
        token_url: String,
        redirect_url: String,
        profile_url: String,
        scope: String,
        exchange_mock_body: String,
        profile_mock_body: String,
        mock_server: mockito::ServerGuard,
    }

    impl Settings {
        fn new() -> Self {
            // Request a new server from the pool
            let server = mockito::Server::new();

            // Use one of these addresses to configure your client
            let host = server.host_with_port();
            let url = server.url();
            let user_profile = UserProfile {
                email: "test_email".to_string(),
            };
            Self {
                client_id: "test_client_id".to_string(),
                client_secret: "test_client_secret".to_string(),
                auth_url: format!("{}/auth_url", url),
                token_url: format!("{}/token_url", url),
                redirect_url: format!("{}/redirect_url", url),
                profile_url: format!("{}/profile_url", url),
                scope: format!("{}/scope_1", url),
                exchange_mock_body: r#"{"access_token":"test_access_token","token_type":"bearer","expires_in":3600,"refresh_token":"test_refresh_token"}"#
                    .to_string(),
                profile_mock_body: r#"{"email":"test_email"}"#.to_string(),
                mock_server: server,
            }
        }
    }

    fn get_base_url(url: &Url) -> String {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or_default(); // Get the host as a str, default to empty string if not present
        format!("{}://{}", scheme, host)
    }

    fn get_base_url_with_path(url: &Url) -> String {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or_default(); // Get the host as a str, default to empty string if not present

        let path = url.path();
        match url.port() {
            Some(port) => format!("{}://{}:{}{}", scheme, host, port, path),
            None => format!("{}://{}{}", scheme, host, path),
        }
    }

    fn create_client() -> OAuth2ClientResult<(AuthorizationCodeClient, Settings)> {
        let settings = Settings::new();
        let credentials = AuthorizationCodeCredentials {
            client_id: settings.client_id.to_string(),
            client_secret: Some(settings.client_id.to_string()),
        };
        let config = AuthorizationCodeUrlConfig {
            auth_url: settings.auth_url.to_string(),
            token_url: Some(settings.token_url.to_string()),
            redirect_url: settings.redirect_url.to_string(),
            profile_url: settings.profile_url.to_string(),
            scopes: vec![settings.scope.to_string()],
        };
        let client = AuthorizationCodeClient::new(credentials, config, None)?;
        Ok((client, settings))
    }

    #[derive(thiserror::Error, Debug)]
    enum TestError {
        #[error(transparent)]
        OAuth2ClientError(#[from] OAuth2ClientError),
        #[error(transparent)]
        ReqwestError(reqwest::Error),
        #[error("Couldnt find {0}")]
        QueryMapError(String),
        #[error("Unable to deserialize profile")]
        ProfileError,
    }

    #[tokio::test]
    async fn get_authorization_url() -> Result<(), TestError> {
        let (mut client, settings) = create_client()?;
        let (url, csrf_token) = client.get_authorization_url();
        let base_url_with_path = get_base_url_with_path(&url);
        // compare between the auth_url with the base url
        assert_eq!(settings.auth_url.to_string(), base_url_with_path);
        let query_map_multi: HashMap<String, Vec<String>> =
            form_urlencoded::parse(url.query().unwrap_or("").as_bytes())
                .into_owned()
                .fold(std::collections::HashMap::new(), |mut acc, (key, value)| {
                    acc.entry(key).or_insert_with(Vec::new).push(value);
                    acc
                });
        // Check response type
        let response_type =
            query_map_multi
                .get("response_type")
                .ok_or(TestError::QueryMapError(
                    "Couldnt find response type".to_string(),
                ))?;
        assert_eq!(response_type[0], "code");
        let client_id = query_map_multi
            .get("client_id")
            .ok_or(TestError::QueryMapError(
                "Couldnt find client id".to_string(),
            ))?;
        assert_eq!(client_id[0], settings.client_id);
        // Check redirect url
        let redirect_url = query_map_multi
            .get("redirect_uri")
            .ok_or(TestError::QueryMapError(
                "Couldnt find redirect url".to_string(),
            ))?;
        assert_eq!(redirect_url[0], settings.redirect_url);
        // Check scopes
        let scopes = query_map_multi
            .get("scope")
            .ok_or(TestError::QueryMapError("Couldnt find scopes".to_string()))?;
        assert_eq!(scopes[0], settings.scope);
        // Check state
        let state = query_map_multi
            .get("state")
            .ok_or(TestError::QueryMapError("Couldnt find state".to_string()))?;
        assert_eq!(state[0], csrf_token.secret().to_owned());
        // Check client id
        Ok(())
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct UserProfile {
        email: String,
    }

    #[tokio::test]
    async fn exchange_code() -> Result<(), TestError> {
        let (mut client, mut settings) = create_client()?;
        // Create a mock for the token exchange
        let exchange_mock = settings
            .mock_server
            .mock("POST", "/token_url")
            .with_status(200)
            .with_body(settings.exchange_mock_body)
            .create();
        // Create a mock for getting profile
        let profile_mock = settings
            .mock_server
            .mock("GET", "/profile_url")
            .with_status(200)
            .with_body(settings.profile_mock_body)
            .create();
        let (url, csrf_token) = client.get_authorization_url();
        let code = "test_code".to_string();
        let state = csrf_token.secret().to_string();
        let csrf_token = csrf_token.secret().to_string();
        let (token, profile) = client
            .verify_user_with_code(code, state, csrf_token)
            .await?;

        // Parse the user's profile
        let profile = profile
            .json::<UserProfile>()
            .await
            .map_err(|_| TestError::ProfileError)?;
        assert_eq!(profile.email, "test_email");
        exchange_mock.assert();
        profile_mock.assert();
        Ok(())
    }
}
