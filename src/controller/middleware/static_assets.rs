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
use tower_http::services::{ServeDir, ServeFile};

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Error, Result};

/// Static asset middleware configuration
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct StaticAssets {
    pub enable: bool,
    /// Check that assets must exist on disk
    pub must_exist: bool,
    /// Assets location
    pub folder: FolderConfig,
    /// Fallback page for a case when no asset exists (404). Useful for SPA
    /// (single page app) where routes are virtual.
    pub fallback: String,
    /// Enable `precompressed_gzip`
    pub precompressed: bool,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct FolderConfig {
    /// Uri for the assets
    pub uri: String,
    /// Path for the assets
    pub path: String,
}

// Implement the MiddlewareTrait for your Middleware struct
impl MiddlewareLayer for StaticAssets {
    /// Returns the name of the middleware.
    fn name(&self) -> &'static str {
        "static_assets"
    }

    /// Checks if the static assets middleware is enabled.
    fn is_enabled(&self) -> bool {
        self.enable
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
        if self.must_exist
            && (!PathBuf::from(&self.folder.path).exists()
                || !PathBuf::from(&self.fallback).exists())
        {
            return Err(Error::Message(format!(
                "one of the static path are not found, Folder `{}` fallback: `{}`",
                self.folder.path, self.fallback,
            )));
        }
        let serve_dir =
            ServeDir::new(&self.folder.path).not_found_service(ServeFile::new(&self.fallback));

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
