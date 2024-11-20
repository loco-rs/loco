use loco_rs::testing;
use {{settings.module_name}}::app::App;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_get_home() {

    testing::request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api").await;

        assert_eq!(res.status_code(), 200);
        res.assert_json(&serde_json::json!({"app_name":"loco"}));
    })
    .await;
}
