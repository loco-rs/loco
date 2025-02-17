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

#[derive(Debug, Deserialize, Serialize)]
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

pub fn get_respond_to(headers: &HeaderMap) -> RespondTo {
    #[allow(clippy::option_if_let_else)]
    if let Some(content_type) = headers.get(CONTENT_TYPE).and_then(|h| h.to_str().ok()) {
        detect_format(content_type)
    } else if let Some(content_type) = headers.get(ACCEPT).and_then(|h| h.to_str().ok()) {
        detect_format(content_type)
    } else {
        RespondTo::None
    }
}

impl<S> FromRequestParts<S> for Format
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        Ok(Self(get_respond_to(&parts.headers)))
    }
}

impl<S> FromRequestParts<S> for RespondTo
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        Ok(get_respond_to(&parts.headers))
    }
}
