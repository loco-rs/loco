use std::{
    collections::HashMap,
    time::{Instant},
};

use oauth2::{
    basic::{BasicClient, },
    reqwest::async_http_client,
    url,
    url::Url,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Response;

use crate::oauth2_storage::error::{OAuth2ClientError, OAuth2ClientResult};

/// A struct that holds the OAuth2 client and the profile URL. - For
/// Authorization Code Grant
pub struct AuthorizationCodeClient {
    pub oauth2: BasicClient,
    pub profile_url: url::Url,
    pub http_client: reqwest::Client,
    pub flow_states: HashMap<String, (PkceCodeVerifier, Instant)>,
}

impl AuthorizationCodeClient {
    /// Create a new instance of `OAuth2Client`.
    pub fn new(
        client_id: String,
        client_secret: Option<String>,
        auth_url: String,
        token_url: Option<String>,
        redirect_url: String,
        profile_url: String,
    ) -> OAuth2ClientResult<Self> {
        let client_id = ClientId::new(client_id);
        let client_secret = if let Some(client_secret) = client_secret {
            Some(ClientSecret::new(client_secret))
        } else {
            None
        };
        let auth_url = AuthUrl::new(auth_url)?;
        let token_url = if let Some(token_url) = token_url {
            Some(TokenUrl::new(token_url)?)
        } else {
            None
        };
        let redirect_url = RedirectUrl::new(redirect_url)?;
        let oauth2 = BasicClient::new(client_id, client_secret, auth_url, token_url)
            .set_redirect_uri(redirect_url);
        let profile_url = url::Url::parse(&profile_url)?;
        Ok(Self {
            oauth2,
            profile_url,
            http_client: reqwest::Client::new(),
            flow_states: HashMap::new(),
        })
    }
}

#[async_trait::async_trait]
pub trait AuthorizationCodeGrantTrait {
    fn get_authorization_code_client(&self) -> AuthorizationCodeClient;
    /// Get authorization URL
    fn get_authorization_url(&self) -> (Url, CsrfToken) {
        let mut client = self.get_authorization_code_client();
        // Remove expired tokens
        client.flow_states.retain(|_, (_, created_at)| {
            created_at.elapsed() < std::time::Duration::from_secs(10 * 60)
        });
        // Generate a PKCE challenge.
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, csrf_token) = client
            .oauth2
            .authorize_url(CsrfToken::new_random)
            // Set the desired scopes.
            .add_scope(Scope::new("read".to_string()))
            .add_scope(Scope::new("write".to_string()))
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
        &self,
        code: String,
        state: String,
        csrf_token: CsrfToken
    ) -> OAuth2ClientResult<Response> {
        let mut client = self.get_authorization_code_client();
        if csrf_token.secret() != state {
            return Err(OAuth2ClientError::CsrfTokenError); 
        }
        // Compare csrf token
        let (pk_verifier, instant) = match client.flow_states.remove(&csrf_token.secret() ) {
            None => {
                return Err(OAuth2ClientError::CsrfTokenError);
            }
            Some(item) => item,
        };

        // Exchange the code with a token
        let token = client
            .oauth2
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pk_verifier)
            .request_async(async_http_client)
            .await?;
        let profile = client
            .http_client
            .get(client.profile_url)
            .bearer_auth(token.access_token().secret().to_owned())
            .send()
            .await?;
        Ok(profile)
    }
}
