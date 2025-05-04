use axum::extract::State;
use loco_rs::{controller::format, prelude::*, tests_cfg};
use rstest::rstest;
use serde::{Deserialize, Serialize};

use crate::infra_cfg;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct MySharedData {
    message: String,
}

struct MySharedDataWithoutClone {
    message: String,
}

#[rstest]
#[case(true)]
#[case(false)]
#[tokio::test]
async fn test_shared_store_extractor(#[case] exists: bool) {
    async fn action(
        State(_ctx): State<AppContext>,
        SharedStore(shared_data): SharedStore<MySharedData>,
    ) -> Result<Response> {
        format::json(&shared_data) // Dereference the RefGuard to get &MySharedData
    }

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    let test_data = MySharedData {
        message: "Hello from SharedStore!".to_string(),
    };
    if exists {
        ctx.shared_store.insert(test_data.clone());
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("Failed to make request");

    if exists {
        assert_eq!(res.status(), axum::http::StatusCode::OK);

        let body: MySharedData = res.json().await.expect("Failed to parse response body");
        assert_eq!(body, test_data);
    } else {
        assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    handle.abort();
}

#[tokio::test]
async fn test_shared_store_without_clone() {
    async fn action(State(ctx): State<AppContext>) -> Result<Response> {
        let shared_data_ref = ctx
            .shared_store
            .get_ref::<MySharedDataWithoutClone>()
            .ok_or_else(|| Error::InternalServerError)?;
        format::text(&shared_data_ref.message)
    }

    let ctx: AppContext = tests_cfg::app::get_app_context().await;

    let test_data = MySharedDataWithoutClone {
        message: "Hello from SharedStore!".to_string(),
    };
    ctx.shared_store.insert(test_data);

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(action), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("Failed to make request");

    assert_eq!(res.status(), axum::http::StatusCode::OK);

    let body = res.text().await.expect("Failed to parse response body");
    assert_eq!(body, "Hello from SharedStore!");

    handle.abort();
}
