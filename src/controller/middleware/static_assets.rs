//! Static Assets Middleware.
//!
//! This middleware serves static files (e.g., images, CSS, JS) from a specified
//! folder to the client. It also allows configuration of a fallback file to
//! serve in case a requested file is not found. Additionally, it can serve
//! precompressed files if enabled via the configuration.
//!
//! The middleware checks if the specified folder and fallback file exist, and
//! if either is missing, it returns an error. If the files exist, the
//! middleware is added to the router to serve static files.

use std::path::PathBuf;

use axum::Router as AXRouter;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

/// Static asset middleware configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaticAssets {
    #[serde(default)]
    pub enable: bool,
    /// Check that assets must exist on disk
    #[serde(default = "default_must_exist")]
    pub must_exist: bool,
    /// Assets location
    #[serde(default = "default_folder_config")]
    pub folder: FolderConfig,
    /// Fallback page for a case when no asset exists. Useful for SPA
    /// (single page app) where routes are virtual.
    #[serde(default = "default_fallback")]
    pub fallback: PathBuf,
    /// Enable `precompressed_gzip`
    #[serde(default = "default_precompressed")]
    pub precompressed: bool,
}

impl Default for StaticAssets {
    fn default() -> Self {
        serde_json::from_value(json!({})).unwrap()
    }
}

fn default_must_exist() -> bool {
    true
}

fn default_precompressed() -> bool {
    false
}

fn default_fallback() -> PathBuf {
    PathBuf::from("assets").join("static").join("404.html")
}

fn default_folder_config() -> FolderConfig {
    FolderConfig {
        uri: "/static".to_string(),
        path: PathBuf::from("assets/static"),
    }
}
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct FolderConfig {
    /// Uri for the assets
    pub uri: String,
    /// Path for the assets
    pub path: PathBuf,
}

// Implement the MiddlewareTrait for your Middleware struct
impl MiddlewareLayer for StaticAssets {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "static"
    }

    /// Checks if the static assets middleware is enabled.
    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    /// Applies the static assets middleware to the application router.
    ///
    /// This method wraps the provided [`AXRouter`] with a service to serve
    /// static files from the folder specified in the configuration. It will
    /// serve a fallback file if the requested file is not found, and can
    /// also serve precompressed (gzip) files if enabled.
    ///
    /// Before applying, it checks if the folder and fallback file exist. If
    /// either is missing, it returns an error.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        if self.must_exist && (!&self.folder.path.exists() || !&self.fallback.exists()) {
            return Err(Error::Message(format!(
                "one of the static path are not found, Folder `{}` fallback: `{}`",
                self.folder.path.display(),
                self.fallback.display(),
            )));
        }

        let serve_dir = ServeDir::new(&self.folder.path).fallback(ServeFile::new(&self.fallback));

        if &self.folder.uri == "/" {
            Ok(app.fallback_service(serve_dir))
        } else {
            Ok(app.nest_service(
                &self.folder.uri,
                if self.precompressed {
                    serve_dir.precompressed_gzip()
                } else {
                    serve_dir
                },
            ))
        }
    }
}
