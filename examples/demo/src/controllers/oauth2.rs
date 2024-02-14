#![allow(clippy::unused_async)]

use axum::{extract::Query, response::Redirect};
use axum_extra::extract::PrivateCookieJar;
use axum_session::{Session, SessionNullPool};
use chrono::{Duration, Local};
use loco_rs::{
    oauth2_store::{basic::BasicTokenResponse, oauth2_grant::OAuth2ClientGrantEnum, TokenResponse},
    prelude::*,
};
use serde::Deserialize;

pub async fn authorization_url(
    State(ctx): State<AppContext>,
    session: Session<SessionNullPool>,
) -> Result<String> {
    let oauth_store = ctx.oauth2.as_ref().unwrap();

    let client = oauth_store.get("google").unwrap();
    let client = match client {
        OAuth2ClientGrantEnum::AuthorizationCode(client) => client,
        _ => {
            return Err(Error::BadRequest("Invalid client type".into()));
        }
    };
    let client = client.clone();
    let mut client = client.lock().await;
    let (auth_url, csrf_token) = client.get_authorization_url();
    let saved_csrf_token = csrf_token.secret().to_owned();

    session.set("CSRF_TOKEN", saved_csrf_token);
    println!("session {:?}", session);

    Ok(format!(
        "<p>Welcome!</p>
    <a href=\"{auth_url}\">
    Click here to sign into Google!
     </a>
        ",
        auth_url = auth_url,
    ))
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct UserProfile {
    email: String,
}

async fn google_callback(
    State(ctx): State<AppContext>,
    session: Session<SessionNullPool>,
    Query(query): Query<AuthRequest>,
    // Extract the private cookie jar from the request
    mut jar: PrivateCookieJar,
) -> Result<impl IntoResponse> {
    let oauth_store = ctx
        .oauth2
        .as_ref()
        .ok_or_else(|| Error::InternalServerError)?;

    let client = oauth_store
        .get("google")
        .ok_or_else(|| Error::InternalServerError)?;
    let client = match client {
        OAuth2ClientGrantEnum::AuthorizationCode(client) => client,
        _ => {
            return Err(Error::BadRequest("Invalid client type".into()));
        }
    };
    let client = client.clone();
    let mut client = client.lock().await;
    // Get the CSRF token from the session
    let csrf_token = session
        .get::<String>("CSRF_TOKEN")
        .ok_or_else(|| Error::BadRequest("CSRF token not found".to_string()))?;
    // Exchange the code with a token
    let (token, profile) = client
        .verify_code_from_callback(query.code, query.state, csrf_token)
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    // Get the user profile
    let profile = profile.json::<UserProfile>().await.unwrap();
    jar = set_token_with_cookie(token, jar);
    Ok((jar, Redirect::to("/protected")))
}

fn set_token_with_cookie(token: BasicTokenResponse, jar: PrivateCookieJar) -> PrivateCookieJar {
    // Set the cookie
    let secs: i64 = token.expires_in().unwrap().as_secs().try_into().unwrap();
    // Create the cookie with the session id, domain, path, and secure flag from
    // the token and profile
    let cookie = axum_extra::extract::cookie::Cookie::build((
        "sid",
        token.access_token().secret().to_owned(),
    ))
    .domain("localhost")
    .path("/")
    // only for testing purposes, toggle this to true in production
    .secure(false)
    .http_only(true)
    .max_age(time::Duration::seconds(secs));
    jar.add(cookie)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("oauth2")
        .add("/", get(authorization_url))
        .add("/google/callback", get(google_callback))
    // .add('/protected', get(protected))
}
