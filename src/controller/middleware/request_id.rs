use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use lazy_static::lazy_static;
use regex::Regex;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LocoRequestId(String);

impl LocoRequestId {
    /// Get the request id
    #[must_use]
    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}

const X_REQUEST_ID: &str = "x-request-id";
const MAX_LEN: usize = 255;
lazy_static! {
    static ref ID_CLEANUP: Regex = Regex::new(r"[^\w\-@]").unwrap();
}

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let header_request_id = request.headers().get(X_REQUEST_ID).cloned();
    let request_id = make_request_id(header_request_id);
    request
        .extensions_mut()
        .insert(LocoRequestId(request_id.clone()));
    let mut res = next.run(request).await;

    if let Ok(v) = HeaderValue::from_str(request_id.as_str()) {
        res.headers_mut().insert(X_REQUEST_ID, v);
    } else {
        warn!("could not set request ID into response headers: `{request_id}`",);
    }
    res
}

fn make_request_id(maybe_request_id: Option<HeaderValue>) -> String {
    maybe_request_id
        .and_then(|hdr| {
            // see: https://github.com/rails/rails/blob/main/actionpack/lib/action_dispatch/middleware/request_id.rb#L39
            let id: Option<String> = hdr.to_str().ok().map(|s| {
                ID_CLEANUP
                    .replace_all(s, "")
                    .chars()
                    .take(MAX_LEN)
                    .collect()
            });
            id.filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use insta::assert_debug_snapshot;

    use super::make_request_id;

    #[test]
    fn create_or_fetch_request_id() {
        let id = make_request_id(Some(HeaderValue::from_static("foo-bar=baz")));
        assert_debug_snapshot!(id);
        let id = make_request_id(Some(HeaderValue::from_static("")));
        assert_debug_snapshot!(id.len());
        let id = make_request_id(Some(HeaderValue::from_static("==========")));
        assert_debug_snapshot!(id.len());
        let long_id = "x".repeat(1000);
        let id = make_request_id(Some(HeaderValue::from_str(&long_id).unwrap()));
        assert_debug_snapshot!(id.len());
        let id = make_request_id(None);
        assert_debug_snapshot!(id.len());
    }
}
