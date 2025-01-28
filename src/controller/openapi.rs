use std::sync::OnceLock;

use axum::{routing::get, Router as AXRouter};
use utoipa::openapi::OpenApi;

use crate::{
    app::AppContext,
    controller::{format, Response},
    Result,
};

static OPENAPI_SPEC: OnceLock<OpenApi> = OnceLock::new();

pub fn set_openapi_spec(api: OpenApi) -> &'static OpenApi {
    OPENAPI_SPEC.get_or_init(|| api)
}

pub fn get_openapi_spec() -> &'static OpenApi {
    OPENAPI_SPEC.get().unwrap()
}

/// Axum handler that returns the `OpenAPI` spec as JSON
pub async fn openapi_spec_json() -> Result<Response> {
    format::json(get_openapi_spec())
}

/// Axum handler that returns the `OpenAPI` spec as YAML
pub async fn openapi_spec_yaml() -> Result<Response> {
    format::yaml(&get_openapi_spec().to_yaml()?)
}

/// Adds the `OpenAPI` endpoints the app router
pub fn add_openapi_endpoints(
    mut app: AXRouter<AppContext>,
    json_url: Option<String>,
    yaml_url: Option<String>,
) -> AXRouter<AppContext> {
    if let Some(json_url) = json_url {
        app = app.route(&json_url, get(openapi_spec_json));
    }
    if let Some(yaml_url) = yaml_url {
        app = app.route(&yaml_url, get(openapi_spec_yaml));
    }
    app
}
