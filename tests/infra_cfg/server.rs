//! # Server Infrastructure Utilities for Loco Framework Testing
//!
//! This module provides utility functions to test a server using the Loco
//! framework. It includes helper functions to start the server from different
//! configurations, such as from boot parameters, application context, or a
//! custom route. These utilities are designed for test environments and use
//! hardcoded ports and bindings.

use loco_rs::{boot, controller::AppRoutes, prelude::*, tests_cfg::db::AppHook};
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
use {serde::Serialize, utoipa::ToSchema, utoipa_axum::routes};

/// The port on which the test server will run.
const TEST_PORT_SERVER: i32 = 5555;

/// The hostname to which the test server binds.
const TEST_BINDING_SERVER: &str = "localhost";

/// Constructs and returns the base URL used for the test server.
pub fn get_base_url() -> String {
    format!("http://{TEST_BINDING_SERVER}:{TEST_PORT_SERVER}/")
}

/// A simple asynchronous handler for GET requests.
async fn get_action() -> Result<Response> {
    format::render().text("text response")
}

/// A simple asynchronous handler for POST requests.
async fn post_action(_body: axum::body::Bytes) -> Result<Response> {
    format::render().text("text response")
}

#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
#[derive(Serialize, Debug, ToSchema)]
pub struct Album {
    title: String,
    rating: u32,
}

#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
#[utoipa::path(
    get,
    path = "/album",
    responses(
        (status = 200, description = "Album found", body = Album),
    ),
)]
async fn get_action_openapi() -> Result<Response> {
    format::json(Album {
        title: "VH II".to_string(),
        rating: 10,
    })
}

/// Starts the server using the provided Loco [`boot::BootResult`] result.
/// It uses hardcoded server parameters such as the port and binding address.
///
/// This function spawns a server task that runs asynchronously and sleeps for 2
/// seconds to ensure the server is fully initialized before handling requests.
pub async fn start_from_boot(boot_result: boot::BootResult) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        boot::start::<AppHook>(
            boot_result,
            boot::ServeParams {
                port: TEST_PORT_SERVER,
                binding: TEST_BINDING_SERVER.to_string(),
            },
            false,
        )
        .await
        .expect("start the server");
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    handle
}

/// Starts the server with a basic route (GET and POST) at the root (`/`), using
/// the given application context.
pub async fn start_from_ctx(ctx: AppContext) -> tokio::task::JoinHandle<()> {
    let app_router = AppRoutes::empty()
        .add_route(
            #[cfg(not(any(
                feature = "openapi_swagger",
                feature = "openapi_redoc",
                feature = "openapi_scalar"
            )))]
            Routes::new()
                .add("/", get(get_action))
                .add("/", post(post_action)),
            #[cfg(any(
                feature = "openapi_swagger",
                feature = "openapi_redoc",
                feature = "openapi_scalar"
            ))]
            Routes::new()
                .add("/", get(get_action))
                .add("/", post(post_action))
                .add("/album", routes!(get_action_openapi)),
        )
        .to_router::<AppHook>(ctx.clone(), axum::Router::new())
        .expect("to router");

    let boot = boot::BootResult {
        app_context: ctx,
        router: Some(app_router),
        run_worker: false,
        run_scheduler: false,
    };

    start_from_boot(boot).await
}

/// Starts the server with a custom route specified by the URI and the HTTP
/// method handler.
pub async fn start_with_route(
    ctx: AppContext,
    uri: &str,
    method: axum::routing::MethodRouter<AppContext>,
) -> tokio::task::JoinHandle<()> {
    let app_router = AppRoutes::empty()
        .add_route(Routes::new().add(uri, method))
        .to_router::<AppHook>(ctx.clone(), axum::Router::new())
        .expect("to router");

    let boot = boot::BootResult {
        app_context: ctx,
        router: Some(app_router),
        run_worker: false,
        run_scheduler: false,
    };
    start_from_boot(boot).await
}
