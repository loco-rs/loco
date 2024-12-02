use axum::http::HeaderMap;
use demo_app::app::App;
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;
use rstest::rstest;
use serial_test::serial;
// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("auth_request");
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
#[case("/response/empty")]
#[case("/response/text")]
#[case("/response/json")]
#[case("/response/empty_json")]
#[case("/response/html")]
#[case("/response/redirect")]
#[case("/response/render_with_status_code")]
#[case("/response/etag")]
#[case("/response/set_cookie")]
#[tokio::test]
#[serial]
async fn can_return_different_responses(#[case] uri: &str) {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get(uri).await;

        let mut headers = HeaderMap::new();
        for (key, value) in response.headers() {
            if ["content-type", "x-powered-by", "location", "etag"].contains(&key.as_str())
                || value.to_str().unwrap().contains("loco-cookie-name")
            {
                headers.insert(key, value.clone());
            }
        }

        assert_debug_snapshot!(
            uri.replace('/', "_"),
            (response.status_code(), response.text(), headers,)
        );
    })
    .await;
}
