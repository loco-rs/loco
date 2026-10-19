use loco_rs::testing::prelude::*;
use app::app::App;
use serial_test::serial;

use super::prepare_data;

#[tokio::test]
#[serial]
async fn can_search_users() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);

        let response = request
            .get("/api/users/search?q=loco")
            .add_header(auth_key, auth_value)
            .await;

        assert_eq!(response.status_code(), 200);

        let body = response.json::<serde_json::Value>();
        assert!(body.get("results").is_some(), "response is a pager envelope");
        // The projection must never carry credentials, whatever the match set.
        let raw = body.to_string();
        assert!(!raw.contains("password"), "response leaks password");
        assert!(!raw.contains("api_key"), "response leaks api_key");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn search_requires_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/users/search?q=loco").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
