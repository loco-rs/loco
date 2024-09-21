#![allow(clippy::unused_async)]
use axum_extra::extract::cookie::Cookie;
use loco_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct Health {
    pub ok: bool,
}

/// return an empty response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn empty() -> Result<Response> {
    format::empty()
}

/// return an text response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn text() -> Result<Response> {
    format::text("Loco")
}

/// return an JSON response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn json() -> Result<Response> {
    format::json(Health { ok: true })
}

/// return an empty JSON response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn empty_json() -> Result<Response> {
    format::empty_json()
}

/// return an  HTML response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn html() -> Result<Response> {
    format::html("hello, world")
}

/// return an redirect response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn redirect() -> Result<Response> {
    format::redirect("/dashboard")
}

/// return an custom status code response
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn render_with_status_code() -> Result<Response> {
    format::render().status(201).empty()
}

/// return response with ETag header
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn etag() -> Result<Response> {
    format::render().etag("loco-etag")?.empty()
}

/// return response with cookie
///
/// # Errors
///
/// This function will return an error if result fails
pub async fn set_cookie() -> Result<Response> {
    let cookie = Cookie::build(("loco-cookie-name", "loco-cookie-value"))
        // .domain("localhost:5173")
        .path("/")
        .same_site(cookie::SameSite::Strict)
        .secure(true)
        .http_only(true)
        .build();

    format::render().cookies(&[cookie])?.json(())
}

pub fn routes() -> Routes<AppContext> {
    Routes::new()
        .prefix("response")
        .add("/empty", get(empty))
        .add("/text", get(text))
        .add("/json", get(json))
        .add("/empty_json", get(empty_json))
        .add("/html", get(html))
        .add("/redirect", get(redirect))
        .add("/render_with_status_code", get(render_with_status_code))
        .add("/etag", get(etag))
        .add("/set_cookie", get(set_cookie))
}
