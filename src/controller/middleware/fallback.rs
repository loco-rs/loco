//! Fallback Middleware
//!
//! This middleware handles fallback logic for the application when routes do
//! not match. It serves a file, a custom not-found message, or a default HTML
//! fallback page based on the configuration.

use axum::{http::StatusCode, response::Html, Router as AXRouter};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeFile;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Fallback {
    /// By default when enabled, returns a prebaked 404 not found page optimized
    /// for development. For production set something else (see fields below)
    pub enable: bool,
    /// For the unlikely reason to return something different than `404`, you
    /// can set it here
    pub code: Option<u16>,
    /// Returns content from a file pointed to by this field with a `404` status
    /// code.
    pub file: Option<String>,
    /// Returns a "404 not found" with a single message string. This sets the
    /// message.
    pub not_found: Option<String>,
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

    /// Applies the fallback middleware to the application router.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let app = if let Some(path) = &self.file {
            app.fallback_service(ServeFile::new(path))
        } else if let Some(not_found) = &self.not_found {
            let not_found = not_found.to_string();
            let code = self
                .code
                .map(StatusCode::from_u16)
                .transpose()
                .map_err(|e| Error::Message(format!("{e}")))?
                .unwrap_or(StatusCode::NOT_FOUND);
            app.fallback(move || async move { (code, not_found) })
        } else {
            let code = self
                .code
                .map(StatusCode::from_u16)
                .transpose()
                .map_err(|e| Error::Message(format!("{e}")))?
                .unwrap_or(StatusCode::NOT_FOUND);
            let content = include_str!("fallback.html");
            app.fallback(move || async move { (code, Html(content)) })
        };
        Ok(app)
    }
}
