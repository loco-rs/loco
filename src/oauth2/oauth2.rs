use oauth2::{
    basic::BasicClient, reqwest::async_http_client, url, AuthUrl, AuthorizationCode, TokenResponse,
};
use reqwest::Response;

use crate::oauth2::error::OAuth2ClientResult;

/// A struct that holds the OAuth2 client and the profile URL.
pub struct OAuth2Client {
    pub oauth2: BasicClient,
    pub profile_url: url::Url,
    pub http_client: reqwest::Client,
}
impl OAuth2Client {
    /// Create a new instance of `OAuth2Client`.
    pub fn new(
        client_id: String,
        client_secret: Option<String>,
        auth_url: String,
        token_url: Option<String>,
        redirect_url: String,
        profile_url: String,
    ) -> OAuth2ClientResult<Self> {
        let client_id = oauth2::ClientId::new(client_id);
        let client_secret = if let Some(client_secret) = client_secret {
            Some(oauth2::ClientSecret::new(client_secret))
        } else {
            None
        };
        let auth_url = AuthUrl::new(auth_url).map_err(|e| e.to_string())?;
        let token_url = if let Some(token_url) = token_url {
            Some(oauth2::TokenUrl::new(token_url)?)
        } else {
            None
        };
        let redirect_url = oauth2::RedirectUrl::new(redirect_url)?;
        let oauth2 = BasicClient::new(client_id, client_secret, auth_url, token_url)
            .set_redirect_uri(redirect_url);
        let profile_url = url::Url::parse(&profile_url)?;

        Ok(Self {
            oauth2,
            profile_url,
            http_client: reqwest::Client::new(),
        })
    }
}
#[async_trait::async_trait]
pub trait OAuth2ClientTrait {
    fn get_oauth2_client(&self) -> OAuth2Client;
    ///
    async fn verify_user_with_code(&self, code: String) -> OAuth2ClientResult<Response> {
        let client = self.get_oauth2_client();
        // Exchange the code with a token
        let token = client
            .oauth2
            .exchange_code(AuthorizationCode::new(code))
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
