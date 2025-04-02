//! # Server Infrastructure Utilities for Loco Framework Testing
//!
//! This module provides utility functions to test a server using the Loco
//! framework. It includes helper functions to start the server from different
//! configurations, such as from boot parameters, application context, or a
//! custom route. These utilities are designed for test environments and use
//! hardcoded ports and bindings.

use loco_rs::{boot, controller::AppRoutes, prelude::*, tests_cfg::db::AppHook};

/// A simple asynchronous handler for GET requests.
async fn get_action() -> Result<Response> {
    format::render().text("text response")
}

/// A simple asynchronous handler for POST requests.
async fn post_action(_body: axum::body::Bytes) -> Result<Response> {
    format::render().text("text response")
}

/// Starts the server using the provided Loco [`boot::BootResult`] result.
/// It uses hardcoded server parameters such as the port and binding address.
///
/// This function spawns a server task that runs asynchronously and sleeps for 2
/// seconds to ensure the server is fully initialized before handling requests.
pub async fn start_from_boot(
    boot_result: boot::BootResult,
    port: Option<i32>,
) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        boot::start::<AppHook>(
            boot_result,
            boot::ServeParams {
                port: port.unwrap_or(TEST_PORT_SERVER),
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
pub async fn start_from_ctx(ctx: AppContext, port: Option<i32>) -> tokio::task::JoinHandle<()> {
    let app_router = AppRoutes::empty()
        .add_route(
            Routes::new()
                .add("/", get(get_action))
                .add("/", post(post_action)),
        )
        .to_router::<AppHook>(ctx.clone(), axum::Router::new())
        .expect("to router");

    let boot = boot::BootResult {
        app_context: ctx,
        router: Some(app_router),
        run_worker: false,
        run_scheduler: false,
    };

    start_from_boot(boot, port).await
}

/// Starts the server with a custom route specified by the URI and the HTTP
/// method handler.
pub async fn start_with_route(
    ctx: AppContext,
    uri: &str,
    method: axum::routing::MethodRouter<AppContext>,
    port: Option<i32>,
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
    start_from_boot(boot, port).await
}
