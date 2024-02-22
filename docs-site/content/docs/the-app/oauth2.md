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

### Configure OAuth2 (Authorization Code Grant)

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

### Initialize OAuth2 (Authorization Code Grant)
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
### OAuth2 Flow (Authorization Code Grant)




### Use OAuth2 (Authorization Code Grant)
