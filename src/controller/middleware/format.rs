//! Detect a content type and format and responds accordingly
use axum::{
    extract::FromRequestParts,
    http::{
        header::{HeaderMap, ACCEPT, CONTENT_TYPE},
        request::Parts,
    },
};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct Format(pub RespondTo);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum RespondTo {
    None,
    Html,
    Json,
    Xml,
    Other(String),
}

fn detect_format(content_type: &str) -> RespondTo {
    if content_type.starts_with("application/json") {
        RespondTo::Json
    } else if content_type.starts_with("text/html") {
        RespondTo::Html
    } else if content_type.starts_with("text/xml")
        || content_type.starts_with("application/xml")
        || content_type.starts_with("application/xhtml")
    {
        RespondTo::Xml
    } else {
        RespondTo::Other(content_type.to_string())
    }
}

/// The most preferred media type of an `Accept` header that names one: ranges in
/// order of their `q` (ties keep their order), skipping `*/*` and `q=0`.
fn accepted(accept: &str) -> Option<RespondTo> {
    let mut ranges: Vec<(f32, &str)> = accept
        .split(',')
        .filter_map(|range| {
            let mut parts = range.split(';');
            let media = parts.next()?.trim();
            let q = parts
                .find_map(|p| p.trim().strip_prefix("q="))
                .and_then(|q| q.trim().parse::<f32>().ok())
                .unwrap_or(1.0);
            (!media.is_empty() && media != "*/*" && q > 0.0).then_some((q, media))
        })
        .collect();
    ranges.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranges.first().map(|(_, media)| detect_format(media))
}

/// The format to respond in: what `Accept` asks for, else the format of the
/// request body (`Content-Type`), else [`RespondTo::None`].
///
/// `Accept` comes first because it is the client's wish for the response;
/// `Content-Type` only describes the body it sent. A form posted from a page
/// (`application/x-www-form-urlencoded`, `Accept: text/html,...`) wants HTML,
/// and a JSON body sent with `Accept: text/html` wants HTML as well.
pub fn get_respond_to(headers: &HeaderMap) -> RespondTo {
    let header = |name| headers.get(name).and_then(|h| h.to_str().ok());
    header(ACCEPT)
        .and_then(accepted)
        .or_else(|| header(CONTENT_TYPE).map(detect_format))
        .unwrap_or(RespondTo::None)
}

impl<S> FromRequestParts<S> for Format
where
    S: Send + Sync,
{
    type Rejection = Error;

    // Header inspection only — see `extractor/auth.rs` for why the `async fn`
    // shape stays anyway.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        Ok(Self(get_respond_to(&parts.headers)))
    }
}

impl<S> FromRequestParts<S> for RespondTo
where
    S: Send + Sync,
{
    type Rejection = Error;

    // As above.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        Ok(get_respond_to(&parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderName, HeaderValue};

    use super::*;

    fn respond_to(pairs: &[(HeaderName, &'static str)]) -> RespondTo {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(name.clone(), HeaderValue::from_static(value));
        }
        get_respond_to(&headers)
    }

    #[test]
    fn accept_decides_over_the_body_type() {
        // a form posted from a browser page
        assert_eq!(
            respond_to(&[
                (CONTENT_TYPE, "application/x-www-form-urlencoded"),
                (
                    ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                ),
            ]),
            RespondTo::Html
        );
        assert_eq!(
            respond_to(&[(CONTENT_TYPE, "application/json"), (ACCEPT, "text/html")]),
            RespondTo::Html
        );
    }

    #[test]
    fn accept_is_read_by_preference() {
        assert_eq!(
            respond_to(&[(ACCEPT, "text/html;q=0.5, application/json")]),
            RespondTo::Json
        );
        assert_eq!(
            respond_to(&[(ACCEPT, "application/json;q=0, text/html")]),
            RespondTo::Html
        );
    }

    #[test]
    fn without_a_specific_accept_the_body_type_decides() {
        assert_eq!(
            respond_to(&[(CONTENT_TYPE, "application/json"), (ACCEPT, "*/*")]),
            RespondTo::Json
        );
        assert_eq!(
            respond_to(&[(CONTENT_TYPE, "application/json")]),
            RespondTo::Json
        );
        assert_eq!(respond_to(&[]), RespondTo::None);
    }
}
