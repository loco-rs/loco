//! Static Assets Embedded Middleware.
//!
//! This middleware serves static files (e.g., images, CSS, JS) from embedded
//! assets built into the binary. It also provides a fallback file to serve in
//! case a requested file is not found.
//!
//! This is particularly useful for distributing single-binary applications
//! with all assets included, eliminating the need for external asset files.

use std::path::PathBuf;

use axum::Router as AXRouter;
use axum::{
    body::Body,
    extract::{Path as AxumPath, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{app::AppContext, controller::middleware::MiddlewareLayer, Result};

// Include the generated static assets at the module level
include!(concat!(env!("OUT_DIR"), "/generated_code/static_assets.rs"));

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

#[derive(Clone)]
pub struct EmbeddedAssets {
    fallback_content: &'static [u8],
}

impl EmbeddedAssets {
    fn new(fallback_path: &str) -> Self {
        tracing::info!(
            "Initializing embedded static assets with fallback path: {}",
            fallback_path
        );

        let assets = get_embedded_static_assets();
        tracing::info!("Loaded {} embedded static assets", assets.len());

        // Log what assets are available
        let available_files: Vec<String> = assets.keys().cloned().collect();
        tracing::info!("Available embedded assets: {:?}", available_files);

        // Try to get the fallback content or use a default empty bytes
        let fallback = assets.get(fallback_path).copied().unwrap_or_else(|| {
            tracing::warn!(
                "Fallback file not found in embedded assets: {}",
                fallback_path
            );

            // Generate a static fallback page
            let fallback_html = concat!(
                "<!DOCTYPE html><html><body>",
                "<h1>404 - Not Found</h1>",
                "</body></html>"
            );

            fallback_html.as_bytes()
        });

        Self {
            fallback_content: fallback,
        }
    }

    fn serve(&self, uri: &str) -> impl IntoResponse {
        let assets = get_embedded_static_assets();

        assets.get(uri).map_or_else(
            || {
                tracing::warn!("Static asset not found: {}, serving fallback", uri);
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "text/html")
                    .body(Body::from(self.fallback_content))
                    .unwrap()
            },
            |content| {
                // Set appropriate content type based on file extension
                let content_type = match uri.rsplit('.').next() {
                    Some("css") => "text/css",
                    Some("js") => "application/javascript",
                    Some("html") => "text/html",
                    Some("png") => "image/png",
                    Some("jpg" | "jpeg") => "image/jpeg",
                    Some("svg") => "image/svg+xml",
                    Some("ico") => "image/x-icon",
                    Some("json") => "application/json",
                    Some("woff") => "font/woff",
                    Some("woff2") => "font/woff2",
                    Some("ttf") => "font/ttf",
                    Some("eot") => "application/vnd.ms-fontobject",
                    Some("otf") => "font/otf",
                    _ => "application/octet-stream",
                };

                tracing::debug!("Serving embedded static asset: {}", uri);
                Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", content_type)
                    .body(Body::from(*content))
                    .unwrap()
            },
        )
    }
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
    /// static files from embedded assets built into the binary.
    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        let fallback_path = format!(
            "/{}",
            self.fallback
                .strip_prefix("assets")
                .unwrap_or(&self.fallback)
                .display()
                .to_string()
                .replace('\\', "/")
        );
        let embedded_assets = EmbeddedAssets::new(&fallback_path);
        let base_uri = self.folder.uri.clone();

        if &base_uri == "/" {
            Ok(app.fallback(move |req: Request| {
                let uri = req.uri().path().to_string();
                let assets = embedded_assets.clone();
                async move { assets.serve(&uri) }
            }))
        } else {
            Ok(app.route(
                &format!("{base_uri}/{{*path}}"),
                get(move |AxumPath(path): AxumPath<String>| {
                    let uri = format!("{base_uri}/{path}");
                    let assets = embedded_assets.clone();
                    async move { assets.serve(&uri) }
                }),
            ))
        }
    }
}
