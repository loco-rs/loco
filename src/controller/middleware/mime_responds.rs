use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use hyper::header::CONTENT_TYPE;
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

#[async_trait]
impl<S> FromRequestParts<S> for Format
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        let headers = &parts.headers;

        let content_type_header = headers.get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                return Ok(Self(RespondTo::Json));
            }

            if content_type.starts_with("text/html") {
                return Ok(Self(RespondTo::Html));
            }
            if content_type.starts_with("text/xml")
                || content_type.starts_with("application/xml")
                || content_type.starts_with("application/xhtml")
            {
                return Ok(Self(RespondTo::Xml));
            }
            return Ok(Self(RespondTo::Other(content_type.to_string())));
        } else {
            return Ok(Self(RespondTo::None));
        }
    }
}
