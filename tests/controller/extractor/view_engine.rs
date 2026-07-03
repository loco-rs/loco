use loco_rs::{prelude::*, tests_cfg};

use crate::infra_cfg;

async fn action(ViewEngine(_engine): ViewEngine<()>) -> Result<Response> {
    format::json(())
}

/// When the `ViewEngine` layer (`Extension<ViewEngine<E>>`) was never
/// installed, the extractor must reject gracefully with an error response
/// instead of panicking.
#[tokio::test]
async fn missing_layer_rejects_gracefully() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("valid response");

    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    handle.abort();
}
