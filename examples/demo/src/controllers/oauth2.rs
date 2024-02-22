#![allow(clippy::unused_async)]

use std::sync::Arc;

use axum::{
    extract::Query,
    response::{Html, Redirect},
};
use axum_extra::extract::PrivateCookieJar;
use axum_session::{Session, SessionNullPool};
use loco_rs::{
    config::{AuthorizationCodeConfig, Oauth2},
    oauth2_store::{
        grants::authorization_code::AuthorizationCodeGrantTrait,
        oauth2_grant::OAuth2ClientGrantEnum, OAuth2ClientStore,
    },
    prelude::*,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    controllers::middleware::auth::{set_token_with_short_live_cookie, OAuth2CookieUser},
    models::{sessions, users, users::OAuthUserProfile},
};

#[derive(Debug, Deserialize)]
pub struct AuthParams {
    code: String,
    state: String,
}

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

fn get_oauth2_authorization_code_config(
    oauth_config: Option<Oauth2>,
    name: &str,
) -> Result<AuthorizationCodeConfig> {
    let oauth_config = oauth_config.ok_or(Error::InternalServerError)?;
    let oauth_config = oauth_config
        .authorization_code
        .iter()
        .find(|c| c.provider_name == name)
        .ok_or(Error::InternalServerError)?;
    Ok(oauth_config.clone())
}

pub async fn authorization_url(
    State(ctx): State<AppContext>,
    session: Session<SessionNullPool>,
) -> Result<Html<String>> {
    let oauth_store = ctx.oauth2.as_ref().unwrap();

    let client = get_oauth2_authorization_code_client(oauth_store, "google")?;
    let mut client = client.lock().await;
    let (auth_url, csrf_token) = client.get_authorization_url();
    session.set("CSRF_TOKEN", csrf_token.secret().to_owned());

    Ok(Html::from(format!(
        "<p>Welcome!</p>
    <a href=\"{auth_url}\">
    Click here to sign into Google!
     </a>
        ",
        auth_url = auth_url,
    )))
}

async fn google_callback(
    State(ctx): State<AppContext>,
    session: Session<SessionNullPool>,
    Query(params): Query<AuthParams>,
    // Extract the private cookie jar from the request
    jar: PrivateCookieJar,
) -> Result<impl IntoResponse> {
    let oauth_store = ctx
        .oauth2
        .as_ref()
        .ok_or_else(|| Error::InternalServerError)?;
    let oauth_config = get_oauth2_authorization_code_config(ctx.config.oauth2, "google")?;
    let client = get_oauth2_authorization_code_client(oauth_store, "google")?;
    let mut client = client.lock().await;
    // Get the CSRF token from the session
    let csrf_token = session
        .get::<String>("CSRF_TOKEN")
        .ok_or_else(|| Error::BadRequest("CSRF token not found".to_string()))?;
    // Exchange the code with a token
    let (token, profile) = client
        .verify_code_from_callback(params.code, params.state, csrf_token)
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    // Get the user profile
    let profile = profile.json::<OAuthUserProfile>().await.unwrap();
    let user = users::Model::upsert_with_oauth(&ctx.db, &profile)
        .await
        .map_err(|_e| {
            tracing::error!("Error creating user");
            Error::InternalServerError
        })?;
    sessions::Model::upsert_with_oauth(&ctx.db, &token, &user)
        .await
        .map_err(|_e| {
            tracing::error!("Error creating session");
            Error::InternalServerError
        })?;

    let jar = set_token_with_short_live_cookie(&oauth_config, token, jar)
        .map_err(|_e| Error::InternalServerError)?;
    let protect_url = &oauth_config
        .cookie_config
        .protected_url
        .clone()
        .unwrap_or("/oauth2/protected".to_string());
    let response = (jar, Redirect::to(protect_url)).into_response();
    tracing::info!("response: {:?}", response);
    Ok(response)
}

async fn protected(user: OAuth2CookieUser) -> Result<impl IntoResponse> {
    let user = user.as_ref();
    Ok("You are protected! Email: ".to_string() + &user.email)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("oauth2")
        .add("/", get(authorization_url))
        .add("/google/callback", get(google_callback))
        .add("/protected", get(protected))
}
