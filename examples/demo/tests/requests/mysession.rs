use axum::http::HeaderName;
use demo_app::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("cache");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn set_request_context_data() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/mysession/request_context").await;

        // Get Cookie from response header
        let headers = response.headers();
        let cookie = headers.get("set-cookie");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "turing");
        assert!(cookie.is_some());
    })
    .await;
}
#[tokio::test]
#[serial]
async fn get_request_context_without_setting_data() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/mysession/request_context").await;
        // Get response body
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "")
    })
    .await;
}

#[tokio::test]
#[serial]
async fn get_request_context_with_setting_data() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/mysession/request_context").await;
        // Get Cookie from response header
        let headers = response.headers();
        let cookie_value = headers.get("set-cookie");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "turing");
        assert!(cookie_value.is_some());
        let data = response.text();

        let response = request
            .get("/mysession/request_context")
            .add_header(
                "cookie".parse::<HeaderName>().unwrap(),
                cookie_value.unwrap().clone(),
            )
            .await;
        // Get response body
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), data);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn remove_request_context_data() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/mysession/request_context").await;
        // Get Cookie from response header
        let headers = response.headers();
        let cookie = headers.get("set-cookie");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "turing");
        assert!(cookie.is_some());
        let response = request
            .get("/mysession/request_context")
            .add_header(
                "cookie".parse::<HeaderName>().unwrap(),
                cookie.unwrap().clone(),
            )
            .await;
        // Get response body
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "turing");
        let response = request
            .delete("/mysession/request_context")
            .add_header(
                "cookie".parse::<HeaderName>().unwrap(),
                cookie.unwrap().clone(),
            )
            .await;
        // Get response body
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "");
    })
    .await;
}
