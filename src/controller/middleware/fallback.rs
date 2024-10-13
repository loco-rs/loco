//! Fallback Middleware
//!
//! This middleware handles fallback logic for the application when routes do
//! not match. It serves a file, a custom not-found message, or a default HTML
//! fallback page based on the configuration.

use axum::{http::StatusCode, response::Html, Router as AXRouter};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::json;
use tower_http::services::ServeFile;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

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
    /// Returns content from a file pointed to by this field with a `404` status
    /// code.
    pub file: Option<String>,
    /// Returns a "404 not found" with a single message string. This sets the
    /// message.
    pub not_found: Option<String>,
}

fn default_status_code() -> StatusCode {
    StatusCode::OK
}

impl Default for Fallback {
    fn default() -> Self {
        serde_json::from_value(json!({})).unwrap()
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
            app.fallback_service(ServeFile::new(path))
        } else if let Some(not_found) = &self.not_found {
            let not_found = not_found.to_string();
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
