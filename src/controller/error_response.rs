//! Lets an app replace the response Loco builds for a [`crate::Error`].
//!
//! `impl IntoResponse for Error` has no access to the app's
//! [`Hooks`], so it stashes the error in the response extensions and the
//! layer in this module hands it to [`Hooks::error_response`].
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};

use super::ErrorDetail;
use crate::{app::AppContext, Error};

/// The error behind a response, as handed to [`Hooks::error_response`].
///
/// [`Hooks::error_response`]: crate::app::Hooks::error_response
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ErrorInfo {
    /// The error returned by the handler or extractor.
    pub error: Arc<Error>,
    /// The status Loco maps this error to by default.
    pub status: StatusCode,
    /// The public-facing detail Loco serializes by default.
    pub detail: ErrorDetail,
}

impl ErrorInfo {
    pub(crate) fn new(error: Error, status: StatusCode, detail: ErrorDetail) -> Self {
        Self {
            error: Arc::new(error),
            status,
            detail,
        }
    }
}

/// The parts of the failed request handed to [`Hooks::error_response`].
///
/// [`Hooks::error_response`]: crate::app::Hooks::error_response
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RequestInfo {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
}

/// [`Hooks::error_response`] as a plain function pointer, so the layer does not
/// have to be generic over `H` (which would force `H: 'static` on callers).
///
/// [`Hooks::error_response`]: crate::app::Hooks::error_response
pub(crate) type HookFn = fn(&ErrorInfo, &RequestInfo, &AppContext) -> Option<Response>;

/// Runs [`Hooks::error_response`] for responses produced from an [`Error`].
///
/// [`Hooks::error_response`]: crate::app::Hooks::error_response
pub(crate) async fn layer(
    State((ctx, hook)): State<(AppContext, HookFn)>,
    req: Request,
    next: Next,
) -> Response {
    // The request is consumed by `next`, so keep what the hook may need.
    let info = RequestInfo {
        method: req.method().clone(),
        uri: req.uri().clone(),
        headers: req.headers().clone(),
    };

    let mut default_response = next.run(req).await;

    // `ErrorInfo` was inserted in the response extensions by `impl IntoResponse for Error`.
    let Some(details) = default_response.extensions_mut().remove::<ErrorInfo>() else {
        return default_response;
    };
    hook(&details, &info, &ctx).unwrap_or(default_response)
}

#[cfg(all(test, feature = "with-db"))]
mod tests {
    use std::path::Path;

    use axum::{
        body::{to_bytes, Body},
        http::{header::CONTENT_TYPE, Request},
        routing::get,
    };
    use rstest::rstest;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::{
        app::Hooks,
        bgworker::Queue,
        boot::{BootResult, StartMode},
        config::Config,
        controller::{AppRoutes, IntoResponse, Json, Routes},
        environment::Environment,
        task::Tasks,
        tests_cfg, Result,
    };

    struct EnvelopeHook;
    #[async_trait::async_trait]
    impl Hooks for EnvelopeHook {
        fn app_name() -> &'static str {
            "TEST"
        }
        fn routes(_ctx: &AppContext) -> AppRoutes {
            AppRoutes::empty()
        }
        async fn boot(_m: StartMode, _e: &Environment, _c: Config) -> Result<BootResult> {
            unimplemented!()
        }
        async fn connect_workers(_ctx: &AppContext, _q: &Queue) -> Result<()> {
            Ok(())
        }
        fn register_tasks(_tasks: &mut Tasks) {}
        async fn truncate(_ctx: &AppContext) -> Result<()> {
            Ok(())
        }
        async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
            Ok(())
        }

        fn error_response(
            err: &ErrorInfo,
            req: &RequestInfo,
            _ctx: &AppContext,
        ) -> Option<Response> {
            let lang = req
                .headers
                .get("accept-language")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("en");
            let body = json!({
                "success": false,
                "code": err.detail.error,
                "is_not_found": matches!(*err.error, Error::NotFound),
                "lang": lang,
            });
            Some((err.status, Json(body)).into_response())
        }
    }

    async fn panics() {
        panic!("boom")
    }

    /// Calls `uri` on an app using `H`; returns status, content type and body.
    async fn call<H: Hooks>(uri: &str) -> (StatusCode, String, String) {
        let fail = |err: fn() -> Error| get(move || async move { Err::<Response, _>(err()) });
        let routes = AppRoutes::empty().add_route(
            Routes::new()
                .add("/ok", get(|| async { "ok" }))
                .add("/missing", fail(|| Error::NotFound))
                .add("/bad", fail(|| Error::BadRequest("nope".to_string())))
                .add("/internal", fail(|| Error::Message("secret".to_string())))
                .add(
                    "/custom",
                    fail(|| {
                        Error::CustomError(
                            StatusCode::IM_A_TEAPOT,
                            ErrorDetail::new("teapot", "short and stout"),
                        )
                    }),
                )
                .add("/panic", get(panics)),
        );
        let ctx = tests_cfg::app::get_app_context().await;
        let router = routes.to_router::<H>(ctx, axum::Router::new()).unwrap();
        let req = Request::builder()
            .uri(uri)
            .header("accept-language", "zh-TW")
            .body(Body::empty())
            .unwrap();
        let res = router.oneshot(req).await.unwrap();
        let status = res.status();
        let content_type = res.headers()[CONTENT_TYPE].to_str().unwrap().to_string();
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    // Without an `error_response` override, every response is byte-for-byte
    // what Loco sent before the hook existed.
    #[rstest]
    #[case("/ok", StatusCode::OK, "text/plain; charset=utf-8", "ok")]
    #[case(
        "/missing",
        StatusCode::NOT_FOUND,
        "application/json",
        r#"{"error":"not_found","description":"Resource was not found"}"#
    )]
    #[case(
        "/bad",
        StatusCode::BAD_REQUEST,
        "application/json",
        r#"{"error":"Bad Request","description":"nope"}"#
    )]
    #[case(
        "/internal",
        StatusCode::INTERNAL_SERVER_ERROR,
        "application/json",
        r#"{"error":"internal_server_error","description":"Internal Server Error"}"#
    )]
    #[case(
        "/custom",
        StatusCode::IM_A_TEAPOT,
        "application/json",
        r#"{"error":"teapot","description":"short and stout"}"#
    )]
    #[tokio::test]
    async fn default_hook_keeps_response(
        #[case] uri: &str,
        #[case] status: StatusCode,
        #[case] content_type: &str,
        #[case] body: &str,
    ) {
        let got = call::<tests_cfg::db::AppHook>(uri).await;
        assert_eq!(got, (status, content_type.to_string(), body.to_string()));
    }

    #[tokio::test]
    async fn hook_replaces_response() {
        let (status, _, body) = call::<EnvelopeHook>("/missing").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap(),
            json!({"success": false, "code": "not_found", "is_not_found": true, "lang": "zh-TW"})
        );
    }

    // A response that did not come from an `Error` never reaches the hook.
    #[tokio::test]
    async fn hook_ignores_successful_response() {
        let (status, _, body) = call::<EnvelopeHook>("/ok").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "ok"));
    }

    // Panics are turned into a response by `catch_panic`, outside the hook's
    // layer, so they keep Loco's default body.
    #[tokio::test]
    async fn panics_keep_default_response() {
        let (status, _, body) = call::<EnvelopeHook>("/panic").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            body,
            r#"{"error":"internal_server_error","description":"Internal Server Error"}"#
        );
    }
}
