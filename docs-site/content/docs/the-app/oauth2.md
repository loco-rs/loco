+++
title = "OAuth2"
description = ""
date = 2024-02-22T08:00:00+00:00
updated = 2024-02-22T08:00:00+00:00
draft = false
weight = 21
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

OAuth2 is a protocol that allows a user to grant a third-party web site or application access to the user's protected resources, without necessarily revealing their long-term credentials or even their identity. For this to work, the user needs to authenticate with the third-party site and grant access to the client application.

Loco supports OAuth2 with the `oauth2` feature. Currently Loco supports the Authorization Code Grant. Client Credentials Grant and Implicit Grant are planned for future releases.

Offical `RFC 6749` OAuth2 documentation can be found [here](https://datatracker.ietf.org/doc/html/rfc6749).\
Shuttle tutorial can be found [here](https://www.shuttle.rs/blog/2023/08/30/using-oauth-with-axum).
## Setup

Add the `oauth2` function as a Hook in the `app.rs` file and import the `oauth2` module from `loco_rs`.

```rust
use loco_rs::oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore};
impl Hooks for App {
    async fn oauth2(
        config: &Config,
        environment: &Environment,
    ) -> Result<Option<OAuth2ClientStore>> {
        Ok(None)
    }
}
```

This hook returns an `OAuth2ClientStore` instance that holds all OAuth2 configurations and grants, covered in the next sections. This `OAuth2ClientStore` instance is stored as part of the application context and is available in controllers, endpoints, task workers, and more.

## Glossary
|                           |                                                                                                          |
|---------------------------|----------------------------------------------------------------------------------------------------------|
| `OAuth2ClientGrantEnum`   | Enum for the different OAuth2 grants, an OAuth2 Client will belong to one of the `OAuth2ClientGrantEnum` |
| `OAuth2ClientStore`       | Abstraction implementation for managing one or more OAuth2 clients.                                      |
| `AuthorizationCodeClient` | A client that uses the Authorization Code Grant.                                                         |

## Configure OAuth2 (Authorization Code Grant)

OAuth2 Configuration is done in the `config/*.yaml` file. The `oauth2` section is used to configure the OAuth2 clients.

This example is using Google Cloud as the OAuth2 provider. You need a Google Cloud project and create OAuth2 credentials for `client_id` and `client_secret`.  `redirect_url` is the server callback endpoint for the provider which should set within `Authorised redirect URIs` section when creating credentials. 

```yaml
# OAuth2 Configuration
oauth2:
 authorization_code: # Authorization code grant type
  - provider_name: google # Identifier for the OAuth2 provider. Replace 'google' with your provider's name if different.
    client_credentials:
      client_id: {{get_env(name="OAUTH_CLIENT_ID", default="oauth_client_id")}} # Replace with your OAuth2 client ID.
      client_secret: {{get_env(name="OAUTH_CLIENT_SECRET", default="oauth_client_secret")}} # Replace with your OAuth2 client secret.
    url_config:
     auth_url: {{get_env(name="AUTH_URL", default="https://accounts.google.com/o/oauth2/auth")}} # authorization endpoint from the provider
     token_url: {{get_env(name="TOKEN_URL", default="https://www.googleapis.com/oauth2/v3/token")}} # token endpoint from the provider for exchanging the authorization code for an access token
     redirect_url: {{get_env(name="REDIRECT_URL", default="http://localhost:3000/oauth2/google/callback")}} # server callback endpoint for the provider
     profile_url: {{get_env(name="PROFILE_URL", default="https://openidconnect.googleapis.com/v1/userinfo")}} # user profile endpoint from the provider for getting user data
     scopes:
      - {{get_env(name="SCOPES_1", default="https://www.googleapis.com/auth/userinfo.email")}} # Scopes for requesting access to user data
      - {{get_env(name="SCOPES_2", default="https://www.googleapis.com/auth/userinfo.profile")}} # Scopes for requesting access to user data
    cookie_config:
      protected_url: {{get_env(name="PROTECTED_URL", default="http://localhost:3000/oauth2/protected")}} # Optional - For redirecting to protect url in cookie to prevent XSS attack
    timeout_seconds: 600 # Optional, default 600 seconds
```

## Initialize OAuth2 (Authorization Code Grant)
Single OAuth2 Authorization Code Client can be initialized with the following hook in `app.rs`:

```rust
use loco_rs::oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore};

async fn oauth2(
    config: &Config,
    environment: &Environment,
) -> Result<Option<OAuth2ClientStore>> {
    // Get the OAuth2 configuration from the config file
    let oauth2_config = config
        .oauth2
        .clone()
        .ok_or(loco_rs::Error::string("Missing configuration for oauth2"))?;
    let authorization_code_grants = oauth2_config.authorization_code;
    // Create a BTreeMap to store the OAuth2 Authorization Code Clients
    let mut clients = BTreeMap::new();
    // Loop through the authorization_code_grants and initialize the OAuth2 Authorization Code Client
    for grant in authorization_code_grants {
        // Initialize the OAuth2 Authorization Code Client
        let client =
            loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeClient::new(
                grant.client_credentials,
                grant.url_config,
                grant.cookie_config,
                None,
            )?;
        // Insert the client into the BTreeMap
        clients.insert(
            grant.provider_name,
            OAuth2ClientGrantEnum::AuthorizationCode(Arc::new(Mutex::new(client))),
        );
    }
    // Create an OAuth2ClientStore with the clients
    let store = OAuth2ClientStore::new(clients);
    // Return the OAuth2ClientStore
    Ok(Some(store))
}
```
## OAuth2 Flow (Authorization Code Grant)
There are 3 entities involved in the OAuth2 Authorization Code Grant flow:

1. Client - `Application server` which requests access to the user's account on the authorization server.
2. Resource Owner - `User` who owns the account on the authorization server. In your example, this would be a user with a Google Account.
3. Authorization Server - `Authorization Server` that hosts the user accounts and can grant access tokens to clients on behalf of the users. In this example, this would be Google Cloud.

### Pre-requisite for the Flow:
`Client Registration`: Before the flow starts, the client (application server) must register with the authorization server. During this registration, the client will typically receive a client_id and client_secret, and will provide the authorization server with one or more redirect_uris. This step is crucial for the authorization server to recognize the client and ensure that the authorization code is sent to the correct callback URL, please set up the callback URL in the `authorization/url_config/redirect_url` field.

### OAuth2 Authorization Code Grant Flow Steps:

1. Authorization Request: The client directs the user (resource owner) to the authorization server's authorization endpoint. This request includes parameters such as the response_type (set to "code" for the authorization code grant), client_id, redirect_uri, scope (which specifies the level of access that the client is requesting), and an optional state parameter (which serves as a CSRF token).


2. User Authentication and Authorization: The user authenticates with the authorization server and decides whether to grant the requested access to the client. The state parameter, if used, is returned to the client in the redirect URI, helping to prevent CSRF attacks.


3. Authorization Response: If the user grants access, the authorization server redirects the user-agent back to the client using the redirect_uri provided earlier, appending an authorization code and the original state value. The authorization code is a temporary code that the client will exchange for an access token.


4. Authorization Code Exchange: The client exchanges the authorization code for an access token (and optionally a refresh token) by making a request to the authorization server's token endpoint. This request includes the grant_type (set to "authorization_code"), code (the authorization code), redirect_uri, and client authentication (typically using the client_id and client_secret as basic auth). 

5. Resource Access: The client uses the access token to access resources on the resource server on behalf of the user.

## Use OAuth2 Example (Authorization Code Grant)
### Setup OAuth2 Store
```rust
// app.rs
impl Hooks for App {
    async fn oauth2(
        config: &Config,
        environment: &Environment,
    ) -> Result<Option<OAuth2ClientStore>> {
        // Get the OAuth2 configuration from the config file
        let oauth2_config = config
            .oauth2
            .clone()
            .ok_or(loco_rs::Error::string("Missing configuration for oauth2"))?;
        // Get the authorization code grants from the configuration
        let authorization_code_grants = oauth2_config.authorization_code;
        // Create a BTreeMap to store the OAuth2 Authorization Code Clients
        let mut clients = BTreeMap::new();
        // Loop through the authorization_code_grants and initialize the OAuth2 Authorization Code Client
        for grant in authorization_code_grants {
            let client =
                AuthorizationCodeClient::new(
                    grant.client_credentials,
                    grant.url_config,
                    grant.cookie_config,
                    None,
                )?;
            // Insert the client into the BTreeMap
            clients.insert(
                grant.provider_name,
                OAuth2ClientGrantEnum::AuthorizationCode(Arc::new(Mutex::new(client))),
            );
        }
        // Create an OAuth2ClientStore with the clients
        let store = OAuth2ClientStore::new(clients);
        Ok(Some(store))
    }
}
```
### Helper Functions
Here is some helper functions in controller to get the OAuth2 Authorization Code Client and the OAuth2 Authorization Code Config from the OAuth2 configuration.
```rust
// controllers/oauth2.rs
use loco_rs::oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore};
use loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeGrantTrait;
use loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeConfig;
use loco_rs::Result;
// Helper function to get the OAuth2 Authorization Code Client from the OAuth2ClientStore
fn get_oauth2_authorization_code_client(
    oauth_store: &Arc<OAuth2ClientStore>,
    name: &str,
) -> Result<Arc<Mutex<dyn AuthorizationCodeGrantTrait>>> {
    let client = oauth_store.get(name).ok_or_else(|| {
        tracing::error!("Client not found");
        Error::InternalServerError
    })?;
    match client {
        OAuth2ClientGrantEnum::AuthorizationCode(client) => Ok(client.clone()),
        _ => {
            tracing::error!("Invalid client type");
            Err(Error::BadRequest("Invalid client type".into()))
        }
    }
}
// Helper function to get the OAuth2 Authorization Code Config from the OAuth2 configuration
fn get_oauth2_authorization_code_config(
    oauth_config: Option<Oauth2>,
    name: &str,
) -> Result<AuthorizationCodeConfig> {
    let oauth_config = oauth_config.ok_or(Error::InternalServerError)?;
    // Get the OAuth2 Authorization Code Config from the OAuth2 configuration with the provider name
    let oauth_config = oauth_config
        .authorization_code
        .iter()
        .find(|c| c.provider_name == name)
        .ok_or(Error::InternalServerError)?;
    Ok(oauth_config.clone())
}
```
### OAuth2 Code Flow
The OAuth2 process requires 2 endpoints and one middleware to be set up in the `controllers` and `controllers/middleware` directories.

Let's start with the authorization endpoint. This endpoint is used to redirect the user to the OAuth2 provider's authorization endpoint. The user will authenticate and authorize the client to access their data.

1. Authorization URL Endpoint
    
    ```rust
    // controllers/oauth2.rs
    use loco_rs::oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore};
    use loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeGrantTrait;

    // GET /oauth2/authorization_url
    pub async fn authorization_url(
        State(ctx): State<AppContext>,
        session: Session<SessionNullPool>,
    ) -> Result<Html<String>> {
        // Get the OAuth2ClientStore from the AppContext
        let oauth_store = ctx.oauth2.as_ref().unwrap();
        // Get the OAuth2 Authorization Code Client from the OAuth2ClientStore using the helper function
        let client = get_oauth2_authorization_code_client(oauth_store, "google")?;
        let mut client = client.lock().await;
        // 1. Authorization Request
        // Construct the authorization URL and generate csrf token using the client
        // auth_url contains the authorization URL, client_id, redirect_uri, scopes and state
        let (auth_url, csrf_token) = client.get_authorization_url();
        // Set the CSRF token in the axum session store
        session.set("CSRF_TOKEN", csrf_token.secret().to_owned());
        // Return the authorization URL for the user to authenticate and authorize the client - this can just return a link and rendered in the frontend
        Ok(Html::from(format!(
            "<p>Welcome!</p>
        <a href=\"{auth_url}\">
        Click here to sign into Google!
         </a>
            ",
            auth_url = auth_url,
        )))
    }
    ```
2. User authorization on OAuth2 provider's page

    User authorizes on the OAuth2 provider's page and redirect back to the client's redirect URL with the authorization code. (Skip)


3. Callback URL Endpoint + 4. Authorization Code Exchange + 5. Resource Access\
    The callback URL endpoint is used to exchange the authorization code for an access token and optionally a refresh token and getting the user profile from the `profile_url`. \
   
   This endpoint is called by the OAuth2 provider after the user has authenticated and authorized the client. This endpoint should be set in the `redirect_url` field in the `config/*.yaml` file.
    ```rust
    // controllers/oauth2.rs
    use loco_rs::oauth2_store::{oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore};
    use loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeGrantTrait;
    use loco_rs::oauth2_store::grants::authorization_code::AuthorizationCodeConfig;
    use serde::Deserialize;
    #[derive(Debug, Deserialize)]
    pub struct AuthParams {
        // The authorization code returned by the OAuth2 provider
        code: String,
        // The state parameter returned by the OAuth2 provider
        state: String,
    }
    
   // GET /oauth2/google/callback
    async fn google_callback(
        State(ctx): State<AppContext>,
        // Get the axum session store
        session: Session<SessionNullPool>,
        Query(params): Query<AuthParams>,
        // Extract the private cookie jar from the request
        jar: PrivateCookieJar,
    ) -> Result<impl IntoResponse> {
        // Get the OAuth2ClientStore from the AppContext
        let oauth_store = ctx
            .oauth2
            .as_ref()
            .ok_or_else(|| Error::InternalServerError)?;
        // Get the OAuth2 Authorization Code Config from the OAuth2 configuration
        let oauth_config = get_oauth2_authorization_code_config(ctx.config.oauth2, "google")?;
       // Get the OAuth2 Authorization Code Client from the OAuth2ClientStore using the helper function 
        let client = get_oauth2_authorization_code_client(oauth_store, "google")?;
        let mut client = client.lock().await;
        // Get the CSRF token from the axum session store
        let csrf_token = session
            .get::<String>("CSRF_TOKEN")
            .ok_or_else(|| Error::BadRequest("CSRF token not found".to_string()))?;
 
        // 4. Authorization Code Exchange + 5. Resource Access
        // This function will also validate the state with the csrf token
        let (token, profile) = client
            .verify_code_from_callback(params.code, params.state, csrf_token)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        // Get the user profile
        let profile = profile.json::<OAuthUserProfile>().await.unwrap();
        
        /* Upsert User - Save user with data if not exist, get user id */
        /* Upsert Sessions - Find session by userId. Update the session_id (token.secret()), expire time and updated time if found, else create new session */
    
        // Create a private cookie with credentials, we put token here but you can also use jwt to replace the token
        // Private cookie jar requires the protect_url must be https to prevent XSS attack
        let jar = set_token_with_short_live_cookie(&oauth_config, token, jar)
            .map_err(|_e| Error::InternalServerError)?;
        // Redirect to the protect url, should be the same as the one in the short lived cookie
        let protect_url = // get the protect url from the cookie_config
        // Redirect to the protect url
        let response = (jar, Redirect::to(protect_url)).into_response();
        Ok(response)
    }
    ```

    Cookie helper function
    ```rust
    // controllers/middleware/auth.rs
    const COOKIE_NAME: &str = "sid";
    // For setting the token in the private cookie jar and redirecting to the protect url
    pub fn set_token_with_short_live_cookie(
        config: &AuthorizationCodeConfig,
        // Can use jwt instead of token
        token: BasicTokenResponse,
        jar: PrivateCookieJar,
    ) -> Result<PrivateCookieJar> {
        // Set the seconds from the token's expires_in field as the max age for the cookie
        let secs: i64 = // get the expires_in from the token, Optional
   
        // protect_url is the url to redirect to after setting the cookie
        let protected_url = // Get protected url from config and parse into url
        let protected_domain = // get the domain from the protected_url
        let protected_path = // get the path from the protected_url
        
        // Create the cookie with the session id, domain, path, and secure flag from
        // the token and profile
        let cookie = axum_extra::extract::cookie::Cookie::build((
            COOKIE_NAME,
            // Save session_id, which is also the credentials. Can use jwt instead of token
            token.access_token().secret().to_owned(),
        ))
        .domain(protected_domain.to_owned())
        .path(protected_domain.to_owned())
        // secure flag is for https - https://datatracker.ietf.org/doc/html/rfc6749#section-3.1.2.1
        .secure(true)
        // Restrict access in the client side code to prevent XSS attacks
        .http_only(true)
        .max_age(time::Duration::seconds(secs));
        Ok(jar.add(cookie))
    }
    ```
   

6. Protected URL Endpoint\
    This will basically be the endpoint that the user will be redirected to after the cookie is set. This endpoint will be used to get the user profile from the OAuth2 provider using the access token from the cookie.
    ```rust
    // controllers/oauth2.rs
    // GET /oauth2/protected
    async fn protected(user: OAuth2CookieUser) -> Result<impl IntoResponse> {
        let user = user.as_ref();
        Ok("You are protected! Email: ".to_string() + &user.email)
    }
    ```
   Cookie middleware \
   this middleware will be highly depended on the data within the short-lived cookie. In our example we are using the access token to get the user profile from the OAuth2 provider.
    ```rust
    // controllers/middleware/auth.rs
    // Middleware to get the user profile from the OAuth2 provider using the access token from the cookie
    const COOKIE_NAME: &str = "sid";

    // Define a struct to extract the cookie from the request
    #[derive(Debug, Deserialize, Serialize)]
    pub struct OAuth2CookieUser {
        pub user: users::Model,
    }
    impl AsRef<users::Model> for OAuth2CookieUser {
        fn as_ref(&self) -> &users::Model {
            &self.user
        }
    } 
    // Middleware to get the user profile from the OAuth2 provider using the access token from the cookie
   // Implement the FromRequestParts trait for the OAuthCookieUser struct
    #[async_trait]
    impl<S> FromRequestParts<S> for OAuth2CookieUser
    where
        S: Send + Sync,
        AppContext: FromRef<S>,
    {
        type Rejection = (StatusCode, String);
        async fn from_request_parts(
            parts: &mut Parts,
            state: &S,
        ) -> core::result::Result<Self, Self::Rejection> {
            let state: AppContext = AppContext::from_ref(state);
            let jar = PrivateCookieJar::from_headers(&parts.headers, state.key.clone());

            let cookie = jar
                .get(COOKIE_NAME)
                .map(|cookie| cookie.value().to_owned())
                .ok_or_else(|| {
                    tracing::info!("Cannot get cookie");
                    (StatusCode::UNAUTHORIZED, "Unauthorized!".to_string())
                })?;
            let user = validate_session_and_retrieve_user(&state.db, &cookie) // find the session by session.session_id, get user from the session.user_id
                .await
                .map_err(|e| {
                    tracing::info!("Cannot validate session");
                    (StatusCode::UNAUTHORIZED, e.to_string())
                })?;
            Ok(Self { user })
        }
    }
    ```
Full example can be found [here](). 