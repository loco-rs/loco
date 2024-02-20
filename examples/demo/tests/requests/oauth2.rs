use std::env;

use blo::{app::App, models::users::OAuthUserProfile};
use loco_rs::testing;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use url::Url;
use wiremock::{
    matchers::{basic_auth, bearer_token, body_string_contains, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct ExchangeMockBody {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
}

struct OAuth2Settings {
    client_id: String,
    client_secret: String,
    code: String,
    auth_url: String,
    token_url: String,
    redirect_url: String,
    profile_url: String,
    protected_url: String,
    scope: String,
    exchange_mock_body: ExchangeMockBody,
    profile_mock_body: OAuthUserProfile,
    mock_server: MockServer,
}

impl OAuth2Settings {
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
        let user_profile = OAuthUserProfile {
            email: "test_email@gmail.com".to_string(),
            name: "test_name".to_string(),
            picture: "test_picture".to_string(),
            sub: "test_sub".to_string(),
            email_verified: true,
            family_name: "test_family_name".to_string(),
            given_name: "test_given_name".to_string(),
            locale: "test_locale".to_string(),
        };
        Self {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            code: "test_code".to_string(),
            auth_url: format!("{}/auth_url", url),
            token_url: format!("{}/token_url", url),
            redirect_url: format!("{}/redirect_url", url),
            profile_url: format!("{}/profile_url", url),
            protected_url: format!("{}/oauth/protected_url", url),
            scope: format!("{}/scope_1", url),
            exchange_mock_body,
            profile_mock_body: user_profile,
            mock_server: server,
        }
    }
}

async fn set_default_url() -> OAuth2Settings {
    let settings = OAuth2Settings::new().await;
    // set environment variables
    // OAUTH_CLIENT_ID
    env::set_var("OAUTH_CLIENT_ID", &settings.client_id);
    // OAUTH_CLIENT_SECRET
    env::set_var("OAUTH_CLIENT_SECRET", &settings.client_secret);
    // AUTH_URL
    env::set_var("AUTH_URL", &settings.auth_url);
    // TOKEN_URL
    env::set_var("TOKEN_URL", &settings.token_url);
    // REDIRECT_URL
    env::set_var("REDIRECT_URL", &settings.redirect_url);
    // PROFILE_URL
    env::set_var("PROFILE_URL", &settings.profile_url);
    // SCOPE_1
    env::set_var("SCOPES_1", &settings.scope);
    // SCOPE_2
    env::set_var("SCOPES_2", &settings.scope);
    // PROTECTED_URL
    env::set_var("PROTECTED_URL", &settings.protected_url);
    settings
}

async fn mock_server(settings: &OAuth2Settings) -> Result<(), Box<dyn std::error::Error>> {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(settings.exchange_mock_body.clone()))
        .expect(1)
        .mount(&settings.mock_server)
        .await;
    // Create a mock for getting profile - https://www.oauth.com/oauth2-servers/access-tokens/access-token-response/
    Mock::given(method("GET"))
        .and(path("/profile_url"))
        .and(bearer_token(
            settings.exchange_mock_body.access_token.clone(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(settings.profile_mock_body.clone()))
        .expect(1)
        .mount(&settings.mock_server)
        .await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_settings() {
    let settings = set_default_url().await;
    assert_eq!(settings.auth_url, env::var("AUTH_URL").unwrap());
    assert_eq!(settings.token_url, env::var("TOKEN_URL").unwrap());
    assert_eq!(settings.redirect_url, env::var("REDIRECT_URL").unwrap());
    assert_eq!(settings.profile_url, env::var("PROFILE_URL").unwrap());
    assert_eq!(settings.scope, env::var("SCOPES_1").unwrap());
    assert_eq!(settings.scope, env::var("SCOPES_2").unwrap());
    assert_eq!(settings.protected_url, env::var("PROTECTED_URL").unwrap());
}

#[tokio::test]
#[serial]
async fn can_authorization_url() -> Result<(), Box<dyn std::error::Error>> {
    let settings = set_default_url().await;
    let assert_html = vec![
        settings.auth_url.clone(),
        serde_urlencoded::to_string([("response_type", "code")])?,
        serde_urlencoded::to_string([("client_id", &settings.client_id)])?,
        serde_urlencoded::to_string([("redirect_uri", &settings.redirect_url)])?,
        serde_urlencoded::to_string([("scope", &settings.scope)])?,
    ];

    testing::request::<App, _, _>(|request, ctx| async move {
        // Test the authorization url
        let res = request.get("/oauth2").await;
        assert_eq!(res.status_code(), 200);
        for url in assert_html {
            assert!(res.text().contains(&url));
        }
    })
    .await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn can_call_google_callback() -> Result<(), Box<dyn std::error::Error>> {
    let settings = set_default_url().await;
    // mock oauth2 server
    mock_server(&settings).await?;
    testing::request::<App, _, _>(|request, ctx| async move {
        // Get the authorization url from the server
        let auth_res = request.get("/oauth2").await;
        // Cookie for csrf token
        let auth_cookie = auth_res.cookies();
        // Get the authorization url from the response HTML
        let mut auth_url = String::new();
        let re = Regex::new(r#"href="([^"]*)""#).unwrap();
        for cap in re.captures_iter(&auth_res.text()) {
            auth_url = cap[1].to_string();
        }
        // Extract the state from the auth_url
        let state = Url::parse(&auth_url)
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.to_string());
        // Test the google callback with csrf token and token
        let res = request
            .get("/oauth2/google/callback")
            .add_query_params(vec![
                ("code", settings.code.clone()),
                ("state", state.unwrap()),
            ])
            .add_cookies(auth_cookie)
            .await;
        assert_eq!(res.status_code(), 303);
        assert_eq!(
            res.headers().get("location").unwrap(),
            &settings.protected_url
        );
    })
    .await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn can_call_protect() -> Result<(), Box<dyn std::error::Error>> {
    let settings = set_default_url().await;
    // mock oauth2 server
    mock_server(&settings).await?;
    testing::request::<App, _, _>(|request, ctx| async move {
        // Get the authorization url from the server
        let auth_res = request.get("/oauth2").await;
        // Cookie for csrf token
        let auth_cookie = auth_res.cookies();
        // Get the authorization url from the response HTML
        let mut auth_url = String::new();
        let re = Regex::new(r#"href="([^"]*)""#).unwrap();
        for cap in re.captures_iter(&auth_res.text()) {
            auth_url = cap[1].to_string();
        }
        // Extract the state from the auth_url
        let state = Url::parse(&auth_url)
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.to_string());
        // Test the google callback with csrf token and token
        let res = request
            .get("/oauth2/google/callback")
            .add_query_params(vec![
                ("code", settings.code.clone()),
                ("state", state.unwrap()),
            ])
            .add_cookies(auth_cookie)
            .await;
        assert_eq!(res.status_code(), 303);
        assert_eq!(
            res.headers().get("location").unwrap(),
            &settings.protected_url
        );
        // Get cookies for private jar
        let cookies = res.cookies();
        // hit the protected url
        let res = request.get("/oauth2/protected").add_cookies(cookies).await;
        assert_eq!(res.status_code(), 200);
        assert!(res.text().contains(&settings.profile_mock_body.email));
    })
    .await;
    Ok(())
}
