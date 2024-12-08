#![allow(clippy::unused_async)]
use axum_extra::extract::cookie::Cookie;
use loco_rs::prelude::*;
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

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

#[derive(Serialize, Debug, ToSchema)]
pub struct Album {
    title: String,
    rating: u32,
}

//
// OpenAPI spec with `utoipa`
//
#[derive(OpenApi)]
#[openapi(paths(album))]
struct Spec;

/// Return an OpenAPI-spec'd response
///
/// # Errors
///
/// This function will return an error if it fails
#[utoipa::path(
    get,
    path = "/response/album",
    responses(
        (status = 200, description = "Album found", body = Album),
    ),
)]
pub async fn album() -> Result<Response> {
    println!("{}", Spec::openapi().to_pretty_json().unwrap());

    format::json(Album {
        title: "VH II".to_string(),
        rating: 10,
    })
}
pub fn routes() -> Routes {
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
        .add("/album", get(album))
        .add("/set_cookie", get(set_cookie))
}
