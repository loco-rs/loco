use loco_rs::testing::prelude::*;
use multitenancy::app::App;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn serves_the_spa_and_browser_route_fallback() {
    request::<App, _, _>(|request, _ctx| async move {
        for path in ["/", "/documents"] {
            let response = request.get(path).await;
            assert_eq!(response.status_code(), 200, "{}", response.text());
            assert!(response
                .text()
                .contains("<title>Loco Multi-tenancy</title>"));
        }
    })
    .await;
}
