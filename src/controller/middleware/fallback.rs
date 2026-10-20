//! Fallback Middleware
//!
//! This middleware handles fallback logic for the application when routes do
//! not match. It serves a file, a custom not-found message, or a default HTML
//! fallback page based on the configuration.

use axum::{
    extract::Request,
    http::StatusCode,
    response::{Html, IntoResponse},
    Router as AXRouter,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tower::Service;
use tower_http::services::ServeFile;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

#[derive(Debug)]
pub struct StatusCodeWrapper(pub StatusCode);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Fallback {
    /// By default when enabled, returns a prebaked 404 not found page optimized
    /// for development. For production set something else (see fields below)
    #[serde(default)]
    pub enable: bool,
    /// For the unlikely reason to return something different than `404`, you
    /// can set it here
    #[serde(
        default = "default_status_code",
        serialize_with = "serialize_status_code",
        deserialize_with = "deserialize_status_code"
    )]
    pub code: StatusCode,
    /// Returns content from a file pointed to by this field, with the status
    /// in `code` (`404` unless you set it).
    pub file: Option<String>,
    /// Returns a "404 not found" with a single message string. This sets the
    /// message.
    pub not_found: Option<String>,
}

fn default_status_code() -> StatusCode {
    StatusCode::NOT_FOUND
}

impl Default for Fallback {
    fn default() -> Self {
        super::defaults()
    }
}

fn deserialize_status_code<'de, D>(de: D) -> Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let code: u16 = Deserialize::deserialize(de)?;
    StatusCode::from_u16(code).map_or_else(
        |_| {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Unsigned(u64::from(code)),
                &"a value between 100 and 600",
            ))
        },
        Ok,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_status_code<S>(status: &StatusCode, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_u16(status.as_u16())
}
impl MiddlewareLayer for Fallback {
    /// Returns the name of the middleware
    fn name(&self) -> &'static str {
        "fallback"
    }

    /// Returns whether the middleware is enabled or not
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the fallback middleware to the application router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let app = if let Some(path) = &self.file {
            let path = path.clone();
            let status_code = self.code;
            app.fallback(move |req: Request| async move {
                // `ServeFile` answers 200 for a file it finds; an unmatched
                // route must still answer with the configured code, like
                // Rails serving `public/404.html`. Reading the file on each
                // request lets you edit it without restarting.
                let mut res = ServeFile::new(path).call(req).await.into_response();
                if res.status().is_success() {
                    *res.status_mut() = status_code;
                }
                res
            })
        } else if let Some(not_found) = &self.not_found {
            let not_found = not_found.clone();
            let status_code = self.code;
            app.fallback(move || async move { (status_code, not_found) })
        } else {
            let content = include_str!("fallback.html");
            let status_code = self.code;
            app.fallback(move || async move { (status_code, Html(content)) })
        };
        Ok(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_code_is_not_found() {
        assert_eq!(Fallback::default().code, StatusCode::NOT_FOUND);
    }
}
