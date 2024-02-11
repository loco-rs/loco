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
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::oauth2_store::error::{OAuth2ClientError, OAuth2ClientResult};

/// A credentials struct that holds the OAuth2 client credentials. - For
/// [`AuthorizationCodeClient`]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationCodeCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// A url config struct that holds the OAuth2 client related URLs. - For
/// [`AuthorizationCodeClient`]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationCodeUrlConfig {
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub profile_url: String,
    pub scopes: Vec<String>,
}

/// [`AuthorizationCodeClient`] that acts as a client for the Authorization Code
/// Grant flow.
pub struct AuthorizationCodeClient {
    /// [`BasicClient`] instance for the OAuth2 client.
    pub oauth2: BasicClient,
    /// [`Url`] instance for the OAuth2 client's profile URL.
    pub profile_url: url::Url,
    /// [`reqwest::Client`] instance for the OAuth2 client's HTTP client.
    pub http_client: reqwest::Client,
    /// A flow states hashMap <CSRF Token, (PKCE Code Verifier, Created time)>
    /// for managing the expiration of the CSRF tokens and PKCE code verifiers.
    pub flow_states: HashMap<String, (PkceCodeVerifier, Instant)>,
    /// A vector of [`Scope`] for the getting the user's profile.
    pub scopes: Vec<Scope>,
    /// A [`std::time::Duration`] for the OAuth2 client's CSRF token timeout
    /// which defaults to 10 minutes (600s).
    pub csrf_token_timeout: std::time::Duration,
}

impl AuthorizationCodeClient {
    /// Create a new instance of [`OAuth2Client`].
    /// # Arguments
    /// * `credentials` - A [`AuthorizationCodeCredentials`] struct that holds
    ///   the OAuth2 client credentials.
    /// * `config` - A [`AuthorizationCodeUrlConfig`] struct that holds the
    ///   OAuth2 client related URLs.
    /// * `timeout_seconds` - An optional timeout in seconds for the csrf token.
    ///   Defaults to 10 minutes (600s).
    /// # Returns
    /// A Result with the [`AuthorizationCodeClient`] instance or an
    /// [`OAuth2ClientError`].
    /// # Example
    /// ```rust,ignore
    /// let credentials = AuthorizationCodeCredentials {
    ///    client_id: "test_client_id".to_string(),
    ///   client_secret: Some("test_client_secret".to_string()),
    /// };
    /// let config = AuthorizationCodeUrlConfig {
    ///     auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
    ///     token_url: Some("https://www.googleapis.com/oauth2/v3/token".to_string()),
    ///     redirect_url: "http://localhost:8000/api/auth/google_callback".to_string(),
    ///     profile_url: "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
    ///     scopes: vec!["https://www.googleapis.com/auth/userinfo.email".to_string()],
    /// };
    /// let client = AuthorizationCodeClient::new(credentials, config, None)?;
    /// ```
    #[must_use]
    pub fn new(
        credentials: AuthorizationCodeCredentials,
        config: AuthorizationCodeUrlConfig,
        timeout_seconds: Option<u64>,
    ) -> OAuth2ClientResult<Self> {
        let client_id = ClientId::new(credentials.client_id);
        let client_secret = credentials.client_secret.map(ClientSecret::new);
        let auth_url = AuthUrl::new(config.auth_url)?;
        let token_url = Some(TokenUrl::new(config.token_url)?);
        let redirect_url = RedirectUrl::new(config.redirect_url)?;
        let oauth2 = BasicClient::new(client_id, client_secret, auth_url, token_url)
            .set_redirect_uri(redirect_url);
        let profile_url = url::Url::parse(&config.profile_url)?;
        let scopes = config
            .scopes
            .iter()
            .map(|scope| Scope::new(scope.to_owned()))
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
    /// Remove expired flow states within the [`AuthorizationCodeClient`].
    /// # Example
    /// ```rust,ignore
    /// client.remove_expire_flow(); // Clear outdated states within client.flow_states
    /// ```
    fn remove_expire_flow(&mut self) {
        // Remove expired tokens
        self.flow_states
            .retain(|_, (_, created_at)| created_at.elapsed() < self.csrf_token_timeout);
    }
    /// Compare two strings in constant time to prevent timing attacks.
    /// # Arguments
    /// * `a` - A string to compare.
    /// * `b` - A string to compare.
    /// # Returns
    /// A boolean value indicating if the strings are equal.
    /// # Example
    /// ```rust,ignore
    /// AuthorizationCodeClient::constant_time_compare("test", "test"); // true
    /// AuthorizationCodeClient::constant_time_compare("test", "test1"); // false
    /// ```
    fn constant_time_compare(a: &str, b: &str) -> bool {
        // Convert the strings to bytes for comparison.
        a.as_bytes().ct_eq(b.as_bytes()).into()
    }
}

#[async_trait::async_trait]
pub trait AuthorizationCodeGrantTrait: Send + Sync {
    /// Get authorization code client
    /// # Returns
    /// A mutable reference to the [`AuthorizationCodeClient`] instance.
    fn get_authorization_code_client(&mut self) -> &mut AuthorizationCodeClient;

    /// Get authorization URL
    /// # Returns
    /// A tuple containing the authorization URL and the CSRF token.
    /// [`Url`] is used to redirect the user to the OAuth2
    /// provider's login page.
    /// [`CsrfToken`] is used to verify the user
    /// when they return to the application. Needs to be stored in the session
    /// or other temporary storage.
    /// # Example
    /// ```rust,ignore
    /// use oauth2::CsrfToken;
    /// use oauth2::url::Url;
    /// use oauth2::basic::BasicClient;
    /// use oauth2::reqwest::async_http_client;
    ///
    ///  // Create a new instance of session store - from tower-sessions
    ///  let session_store = MemoryStore::default();
    ///  // Create a new instance of `SessionManagerLayer` with the session store for axum layer
    ///  let session_layer = SessionManagerLayer::new(session_store)
    ///     // This is needed because the oauth2 client callback request is coming from a different domain, but be careful with this in production since it can be a security risk.
    ///     .with_same_site(SameSite::Lax);
    ///  // Create a new instance of `OAuth2ClientStore` with the `AuthorizationCodeClient` instance
    ///  let client = AuthorizationCodeClient::new();
    ///  let mut clients = BTreeMap::new();
    ///  clients.insert(
    ///    "google".to_string(),
    ///    OAuth2ClientGrantEnum::AuthorizationCode(Arc::new(Mutex::new(authorization_code_client))),
    ///  );
    ///  let mut client_store = OAuth2ClientStore::new(clients);
    ///  let app = Router::new().route("/auth/google", get(get_auth_url)).layer(Extension(Arc::new(client_store))).layer(session_layer);
    ///
    ///  pub async fn get_auth_url(Extension(session_store): Extension<Session>, Extension(oauth_client_store): Extension<Arc<OAuth2ClientStore>>,) -> Url {
    ///     let client = oauth_client_store.get("google").unwrap();
    ///     // Get the authorization URL and the CSRF token
    ///     let (auth_url, csrf_token) = client.get_authorization_url();
    ///
    ///     // Save the CSRF token in the session store
    ///     session_store
    ///         .insert("csrf_token", saved_csrf_token)
    ///         .await
    ///         .unwrap();
    ///     // redirect the user to the authorization URL
    ///     Ok(auth_url)
    /// }
    /// ```
    #[must_use]
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
        client
            .flow_states
            .insert(csrf_token.secret().clone(), (pkce_verifier, Instant::now()));
        (auth_url, csrf_token)
    }
    /// Verify code from the provider callback request after returns from the
    /// OAuth2 provider's login page.
    /// # Arguments
    /// * `code` - A string containing the code returned from the OAuth2
    ///   provider callback request query.
    /// * `state` - A string containing the state returned from the OAuth2
    ///   provider response which extracted from the provider callback request
    ///   query.
    /// * `csrf_token` - A string containing the CSRF token saved in the
    ///   temporary session after the
    ///   [`AuthorizationCodeClient::get_authorization_url`] method.
    /// # Returns
    /// A tuple containing the token response and the profile response.
    /// [`BasicTokenResponse`] is the token response from the OAuth2 provider.
    /// [`Response`] is the profile response from the OAuth2 provider which
    /// describes the user's profile. This response json information will be
    /// determined by [`AuthorizationCodeClient::scopes`] # Errors
    /// An [`OAuth2ClientError::CsrfTokenError`] if the csrf token is invalid.
    /// An [`OAuth2ClientError::BasicTokenError`] if the token
    /// exchange fails.
    /// An [`OAuth2ClientError::ProfileError`] if the profile request fails.
    /// # Example
    /// ```rust,ignore
    /// use std::collections::BTreeMap;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use axum::{Extension, Router};
    /// use axum::extract::Query;
    /// use axum::response::Redirect;
    /// use axum::routing::get;use oauth2::CsrfToken;
    /// use oauth2::url::Url;
    /// use oauth2::basic::BasicClient;
    /// use oauth2::reqwest::async_http_client;
    /// use serde::Deserialize;
    /// use tokio::sync::Mutex;
    /// use super::*;
    ///
    ///  // Create a new instance of session store - from tower-sessions
    ///  let session_store = MemoryStore::default();
    ///  // Create a new instance of `SessionManagerLayer` with the session store for axum layer
    ///  let session_layer = SessionManagerLayer::new(session_store)
    ///     // This is needed because the oauth2 client callback request is coming from a different domain, but be careful with this in production since it can be a security risk.
    ///     .with_same_site(SameSite::Lax);
    ///  // Create a new instance of `OAuth2ClientStore` with the `AuthorizationCodeClient` instance
    ///  let client = AuthorizationCodeClient::new();
    ///  let mut clients = BTreeMap::new();
    ///  clients.insert(
    ///    "google".to_string(),
    ///    OAuth2ClientGrantEnum::AuthorizationCode(Arc::new(Mutex::new(authorization_code_client))),
    ///  );
    ///  let mut client_store = OAuth2ClientStore::new(clients);
    ///  let app = Router::new().route("/auth/google_callback", get(google_callback)).layer(Extension(Arc::new(client_store))).layer(session_layer);
    ///
    ///  #[derive(Debug, Deserialize)]
    ///  pub struct AuthRequest {
    ///     code: String,
    ///     state: String,
    ///  }
    ///  #[derive(Deserialize, Clone, Debug)]
    ///  pub struct UserProfile {
    ///     email: String,
    ///  }
    ///  pub async fn google_callback(Extension(session_store): Extension<Session>, Extension(oauth_client_store): Extension<Arc<OAuth2ClientStore>>, Query(query): Query<AuthRequest>, jar: PrivateCookieJar) -> String {
    ///     // Get the previous stored csrf_token from the store
    ///     let csrf_token = session_store.get::<String>("csrf_token").await.unwrap();
    ///     // Get the client from the store
    ///     let client = oauth_client_store.get("google").unwrap();
    ///     // Get the token and profile from the client   
    ///     let (token, profile) = client.verify_code_from_callback(query.code, query.state, csrf_token).await.unwrap();
    ///     // Parse the user's profile
    ///     let profile = profile.json::<UserProfile>().await.unwrap();
    ///     let secs: i64 = token.access_token().expires_in().as_secs().try_into().unwrap();
    ///     // Create a new user based on user's profile into your database
    ///     // Create a cookie for the user's session
    ///     let cookie = axum_extra::extract::cookie::Cookie::build(("sid", db_user_id))
    ///                                 .domain("localhost")
    ///                                 // only for testing purposes, toggle this to true in production
    ///                                 .secure(false)
    ///                                 .http_only(true)
    ///                                 .max_age(Duration::seconds(secs));
    ///      // Redirect the user to the protected route
    ///      let jar = jar.add(cookie);
    ///      Ok((jar, Redirect::to("/protected")))
    /// }
    ///     
    #[must_use]
    async fn verify_code_from_callback(
        &mut self,
        code: String,
        state: String,
        csrf_token: String,
    ) -> OAuth2ClientResult<(BasicTokenResponse, Response)> {
        let client = self.get_authorization_code_client();
        // Clear outdated flow states
        client.remove_expire_flow();
        // Compare csrf token, use subtle to prevent time attack
        if !AuthorizationCodeClient::constant_time_compare(&csrf_token, &state) {
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
            .await
            .map_err(OAuth2ClientError::ProfileError)?;
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
    use wiremock::{
        matchers::{basic_auth, bearer_token, body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    struct Settings {
        client_id: String,
        client_secret: String,
        code: String,
        auth_url: String,
        token_url: String,
        redirect_url: String,
        profile_url: String,
        scope: String,
        exchange_mock_body: ExchangeMockBody,
        profile_mock_body: UserProfile,
        mock_server: MockServer,
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    struct ExchangeMockBody {
        access_token: String,
        token_type: String,
        expires_in: u64,
        refresh_token: String,
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    struct UserProfile {
        email: String,
    }

    impl Settings {
        async fn new() -> Self {
            // Request a new server from the pool
            let server = MockServer::start().await;

            // Use one of these addresses to configure your client
            let url = server.uri();
            let exchange_mock_body = ExchangeMockBody {
                access_token: "test_access_token".to_string(),
                token_type: "bearer".to_string(),
                expires_in: 3600,
                refresh_token: "test_refresh_token".to_string(),
            };
            let user_profile = UserProfile {
                email: "test_email".to_string(),
            };
            Self {
                client_id: "test_client_id".to_string(),
                client_secret: "test_client_secret".to_string(),
                code: "test_code".to_string(),
                auth_url: format!("{}/auth_url", url),
                token_url: format!("{}/token_url", url),
                redirect_url: format!("{}/redirect_url", url),
                profile_url: format!("{}/profile_url", url),
                scope: format!("{}/scope_1", url),
                exchange_mock_body,
                profile_mock_body: user_profile,
                mock_server: server,
            }
        }
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

    async fn create_client() -> OAuth2ClientResult<(AuthorizationCodeClient, Settings)> {
        let settings = Settings::new().await;
        let credentials = AuthorizationCodeCredentials {
            client_id: settings.client_id.to_string(),
            client_secret: Some(settings.client_secret.to_string()),
        };
        let config = AuthorizationCodeUrlConfig {
            auth_url: settings.auth_url.to_string(),
            token_url: settings.token_url.to_string(),
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
        #[error("Mock json data parse Error")]
        MockJsonDataError(#[from] serde_json::Error),
        #[error("Mock form data error")]
        MockFormDataError(#[from] serde_urlencoded::ser::Error),
    }

    #[tokio::test]
    async fn get_authorization_url() -> Result<(), TestError> {
        let (mut client, settings) = create_client().await?;
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

    #[tokio::test]
    async fn exchange_code() -> Result<(), TestError> {
        let (mut client, settings) = create_client().await?;
        let token_form_body = vec![
            serde_urlencoded::to_string([("code", &settings.code)])?,
            serde_urlencoded::to_string([("redirect_uri", &settings.redirect_url)])?,
            serde_urlencoded::to_string([("grant_type", "authorization_code")])?,
        ];
        // Create a mock for the token exchange - https://www.oauth.com/oauth2-servers/access-tokens/authorization-code-request/
        let mut token_mock = Mock::given(method("POST"))
            .and(path("/token_url"))
            // Client Authorization Auth Header from RFC6749(OAuth2) - https://datatracker.ietf.org/doc/html/rfc6749#section-2.3
            .and(basic_auth(
                settings.client_id.clone(),
                settings.client_secret.clone(),
            ));
        // Access Token Request Body from RFC6749(OAuth2) - https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.3
        for url in token_form_body {
            token_mock = token_mock.and(body_string_contains(url));
        }
        token_mock
            .respond_with(
                ResponseTemplate::new(200).set_body_json(settings.exchange_mock_body.clone()),
            )
            .expect(1)
            .mount(&settings.mock_server)
            .await;
        // Create a mock for getting profile - https://www.oauth.com/oauth2-servers/access-tokens/access-token-response/
        Mock::given(method("GET"))
            .and(path("/profile_url"))
            .and(bearer_token(
                settings.exchange_mock_body.access_token.clone(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(settings.profile_mock_body))
            .expect(1)
            .mount(&settings.mock_server)
            .await;
        let (_url, csrf_token) = client.get_authorization_url();

        let state = csrf_token.secret().to_string();
        let csrf_token = csrf_token.secret().to_string();
        let (_token, profile) = client
            .verify_code_from_callback(settings.code, state, csrf_token)
            .await?;

        // Parse the user's profile
        let profile = profile
            .json::<UserProfile>()
            .await
            .map_err(|_| TestError::ProfileError)?;
        assert_eq!(profile.email, "test_email");
        Ok(())
    }
}
