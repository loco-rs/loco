//! This module contains utility functions for generating HTTP responses that
//! are commonly used in web applications. These functions simplify the process
//! of creating responses with various data types.
//!
//! # Example:
//!
//! This example illustrates how to construct a JSON-formatted response using a
//! Rust struct.
//!
//! ```rust
//! use loco_rs::prelude::*;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! pub struct Health {
//!     pub ok: bool,
//! }
//!
//! async fn ping() -> Result<Response> {
//!    format::json(Health { ok: true })
//! }
//! ```

use axum::{
    body::Body,
    http::{response::Builder, HeaderName, HeaderValue},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::Cookie;
use bytes::{BufMut, BytesMut};
use hyper::{header, StatusCode};
use serde::Serialize;
use serde_json::json;

use crate::{
    controller::{
        views::{self, ViewRenderer},
        Json,
    },
    Result,
};

/// Returns an empty response.
///
/// # Example:
///
/// This example illustrates how to return an empty response.
/// ```rust
/// use loco_rs::prelude::*;
///
/// async fn endpoint() -> Result<Response> {
///    format::empty()
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn empty() -> Result<Response> {
    Ok(().into_response())
}

/// Returns a response containing the provided text.
///
/// # Example:
///
/// This example illustrates how to return an text response.
/// ```rust
/// use loco_rs::prelude::*;
///
/// async fn endpoint() -> Result<Response> {
///    format::text("MESSAGE-RESPONSE")
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn text(t: &str) -> Result<Response> {
    Ok(t.to_string().into_response())
}

/// Returns a JSON response containing the provided data.
///
/// # Example:
///
/// This example illustrates how to construct a JSON-formatted response using a
/// Rust struct.
///
/// ```rust
/// use loco_rs::prelude::*;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// pub struct Health {
///     pub ok: bool,
/// }
///
/// async fn endpoint() -> Result<Response> {
///    format::json(Health { ok: true })
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn json<T: Serialize>(t: T) -> Result<Response> {
    Ok(Json(t).into_response())
}

/// Respond with empty json (`{}`)
///
/// # Errors
///
/// This function will return an error if serde fails
pub fn empty_json() -> Result<Response> {
    json(json!({}))
}

/// Returns an HTML response
///
/// # Example:
///
/// ```rust
/// use loco_rs::prelude::*;
///
/// async fn endpoint() -> Result<Response> {
///    format::html("hello, world")
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn html(content: &str) -> Result<Response> {
    Ok(Html(content.to_string()).into_response())
}

/// Returns an redirect response
///
/// # Example:
///
/// ```rust
/// use loco_rs::prelude::*;
///
/// async fn login() -> Result<Response> {
///    format::redirect("/dashboard")
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn redirect(to: &str) -> Result<Response> {
    Ok(Redirect::to(to).into_response())
}

/// Render template located by `key`
///
/// # Errors
///
/// This function will return an error if rendering fails
pub fn view<V, S>(v: &V, key: &str, data: S) -> Result<Response>
where
    V: ViewRenderer,
    S: Serialize,
{
    let res = v.render(key, data)?;
    html(&res)
}

/// Render template from string
///
/// # Errors
///
/// This function will return an error if rendering fails
pub fn template<S>(template: &str, data: S) -> Result<Response>
where
    S: Serialize,
{
    html(&views::template(template, data)?)
}

#[derive(Debug)]
pub struct RenderBuilder {
    response: Builder,
}

impl RenderBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            response: Builder::new().status(StatusCode::OK),
        }
    }

    /// Get an Axum response builder (escape hatch, leaving this builder)
    #[must_use]
    pub fn response(self) -> Builder {
        self.response
    }

    /// Add a status code
    #[must_use]
    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<axum::http::Error>,
    {
        Self {
            response: self.response.status(status),
        }
    }

    /// Add a single header
    #[must_use]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<axum::http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<axum::http::Error>,
    {
        Self {
            response: self.response.header(key, value),
        }
    }

    /// Add an etag
    ///
    /// # Errors
    ///
    /// This function will return an error if provided etag value is illegal
    /// (not visible ASCII)
    pub fn etag(self, etag: &str) -> Result<Self> {
        Ok(Self {
            response: self
                .response
                .header(header::ETAG, HeaderValue::from_str(etag)?),
        })
    }

    /// Add a collection of cookies to the response
    ///
    /// # Errors
    /// Returns error if cookie values are illegal
    pub fn cookies(self, cookies: &[Cookie<'_>]) -> Result<Self> {
        let mut res = self.response;
        for cookie in cookies {
            let header_value = cookie.encoded().to_string().parse::<HeaderValue>()?;
            res = res.header(header::SET_COOKIE, header_value);
        }
        Ok(Self { response: res })
    }

    /// Finalize and return a text response
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn text(self, content: &str) -> Result<Response> {
        Ok(self
            .response
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(Body::from(content.to_string()))?)
    }

    /// Finalize and return an empty response
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn empty(self) -> Result<Response> {
        Ok(self.response.body(Body::empty())?)
    }

    /// Render template located by `key`
    ///
    /// # Errors
    ///
    /// This function will return an error if rendering fails
    pub fn view<V, S>(self, v: &V, key: &str, data: S) -> Result<Response>
    where
        V: ViewRenderer,
        S: Serialize,
    {
        let content = v.render(key, data)?;
        self.html(&content)
    }

    /// Render template located by `key`
    ///
    /// # Errors
    ///
    /// This function will return an error if rendering fails
    pub fn template<S>(self, template: &str, data: S) -> Result<Response>
    where
        S: Serialize,
    {
        html(&views::template(template, data)?)
    }

    /// Finalize and return a HTML response
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn html(self, content: &str) -> Result<Response> {
        Ok(self
            .response
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            )
            .body(Body::from(content.to_string()))?)
    }

    /// Finalize and return a JSON response
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn json<T>(self, item: T) -> Result<Response>
    where
        T: Serialize,
    {
        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, &item)?;
        let body = Body::from(buf.into_inner().freeze());
        Ok(self
            .response
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
            )
            .body(body)?)
    }

    /// Finalize and redirect request
    ///
    /// # Errors
    ///
    /// This function will return an error if IO fails
    pub fn redirect(self, to: &str) -> Result<Response> {
        Ok(self
            .response
            .status(StatusCode::SEE_OTHER)
            .header(header::LOCATION, to)
            .body(Body::empty())?)
    }
}

impl Default for RenderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn render() -> RenderBuilder {
    RenderBuilder::new()
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{controller::views::engines::TeraView, prelude::*};
    use insta::assert_debug_snapshot;
    use tree_fs;

    async fn response_body_to_string(response: hyper::Response<Body>) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), 200)
            .await
            .unwrap();
        std::str::from_utf8(&bytes).unwrap().to_string()
    }

    pub fn get_header_from_response(
        response: &hyper::Response<Body>,
        header: &str,
    ) -> Option<String> {
        Some(response.headers().get(header)?.to_str().ok()?.to_string())
    }

    #[tokio::test]
    async fn empty_response_format() {
        let response: hyper::Response<Body> = empty().unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, String::new());
    }

    #[tokio::test]
    async fn text_response_format() {
        let response_content = "loco";
        let response = text(response_content).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, response_content);
    }

    #[tokio::test]
    async fn json_response_format() {
        let response_content = serde_json::json!({"loco": "app"});
        let response = json(&response_content).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(
            response_body_to_string(response).await,
            response_content.to_string()
        );
    }

    #[tokio::test]
    async fn empty_json_response_format() {
        let response = empty_json().unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(
            response_body_to_string(response).await,
            serde_json::json!({}).to_string()
        );
    }

    #[tokio::test]
    async fn html_response_format() {
        let response_content: &str = "<h1>loco</h1>";
        let response = html(response_content).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, response_content);
    }

    #[tokio::test]
    async fn redirect_response() {
        let response = redirect("https://loco.rs").unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, String::new());
    }

    #[tokio::test]
    async fn view_response() {
        let yaml_content = r"
        files:
        - path: template/test.html
          content: |-
            - {{foo}}
        ";

        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();
        let v = TeraView::from_custom_dir(&tree_res).unwrap();

        assert_debug_snapshot!(view(&v, "template/none.html", serde_json::json!({})));
        let response = view(&v, "template/test.html", serde_json::json!({"foo": "loco"})).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, "- loco");
    }

    #[tokio::test]
    async fn template_response() {
        let response = template("- {{foo}}", serde_json::json!({"foo": "loco"})).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, "- loco");
    }

    #[tokio::test]
    async fn builder_set_status_code_response() {
        assert_eq!(render().empty().unwrap().status(), 200);
        assert_eq!(render().status(202).empty().unwrap().status(), 202);
    }

    #[tokio::test]
    async fn builder_set_headers_response() {
        assert_eq!(render().empty().unwrap().headers().len(), 0);
        let response = render()
            .header("header-1", "loco")
            .header("header-2", "rs")
            .empty()
            .unwrap();

        assert_eq!(response.headers().len(), 2);
        assert_eq!(
            get_header_from_response(&response, "header-1"),
            Some("loco".to_string())
        );
        assert_eq!(
            get_header_from_response(&response, "header-2"),
            Some("rs".to_string())
        );
    }

    #[tokio::test]
    async fn builder_etag_response() {
        assert_eq!(render().empty().unwrap().headers().len(), 0);
        let response = render().etag("foobar").unwrap().empty().unwrap();

        assert_eq!(response.headers().len(), 1);
        assert_eq!(
            get_header_from_response(&response, "etag"),
            Some("foobar".to_string())
        );
    }

    #[tokio::test]
    async fn builder_cookies_response() {
        let response = render()
            .cookies(&[
                cookie::Cookie::new("foo", "bar"),
                cookie::Cookie::new("baz", "qux"),
            ])
            .unwrap()
            .empty()
            .unwrap();

        assert_debug_snapshot!(response.headers());
    }

    #[tokio::test]
    async fn builder_text_response() {
        let response = render().text("loco").unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, "loco");
    }

    #[tokio::test]
    async fn builder_empty_response() {
        let response = render().empty().unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, String::new());
    }

    #[tokio::test]
    async fn builder_view_response() {
        let yaml_content = r"
        files:
        - path: template/test.html
          content: |-
            - {{foo}}
        ";

        let tree_res = tree_fs::from_yaml_str(yaml_content).unwrap();
        let v = TeraView::from_custom_dir(&tree_res).unwrap();

        assert_debug_snapshot!(view(&v, "template/none.html", serde_json::json!({})));
        let response = render()
            .view(&v, "template/test.html", serde_json::json!({"foo": "loco"}))
            .unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, "- loco");
    }

    #[tokio::test]
    async fn builder_template_response() {
        let response = render()
            .template("- {{foo}}", serde_json::json!({"foo": "loco"}))
            .unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, "- loco");
    }

    #[tokio::test]
    async fn builder_html_response() {
        let response_content = "<h1>loco</h1>";
        let response = render().html(response_content).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(&response_body_to_string(response).await, response_content);
    }

    #[tokio::test]
    async fn builder_json_response() {
        let response_content = serde_json::json!({"loco": "app"});
        let response = render().json(&response_content).unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(
            response_body_to_string(response).await,
            response_content.to_string()
        );
    }

    #[tokio::test]
    async fn builder_redirect_response() {
        let response = render().redirect("https://loco.rs").unwrap();

        assert_debug_snapshot!(response);
        assert_eq!(response_body_to_string(response).await, String::new());
    }
}
