use crate::infra_cfg;
use loco_rs::{prelude::*, tests_cfg};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use validator::Validate;

#[tokio::test]
#[serial]
async fn json_rejection() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    #[derive(Debug, Deserialize, Serialize, Validate)]
    pub struct Data {
        #[validate(length(min = 5, message = "message_str"))]
        pub name: String,
        #[validate(email)]
        pub email: String,
    }

    #[allow(clippy::items_after_statements)]
    async fn action(JsonValidate(_params): JsonValidate<Data>) -> Result<Response> {
        format::json(())
    }

    let handle = infra_cfg::server::start_with_route(ctx, "/", post(action)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(infra_cfg::server::get_base_url())
        .json(&serde_json::json!({"name": "test", "email": "invalid"}))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 400);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!(
        {
            "errors":{
                "email":[{"code":"email","message":null,"params":{"value":"invalid"}}],
                "name":[{"code":"length","message":"message_str","params":{"min":5,"value":"test"}}]
        }
    });

    assert_eq!(res_json, expected_json);

    handle.abort();
}
