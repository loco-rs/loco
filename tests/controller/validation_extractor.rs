use loco_rs::{prelude::*, tests_cfg};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use validator::Validate;

use crate::infra_cfg;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Data {
    #[validate(length(min = 5, message = "message_str"))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

async fn validation_with_response(
    JsonValidateWithMessage(_params): JsonValidateWithMessage<Data>,
) -> Result<Response> {
    format::json(())
}

async fn simple_validation(JsonValidate(_params): JsonValidate<Data>) -> Result<Response> {
    format::json(())
}

#[tokio::test]
#[serial]
async fn can_validation_with_response() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", post(validation_with_response), Some(port))
            .await;

    let client = reqwest::Client::new();
    let res = client
        .post(get_base_url_port(port))
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

#[tokio::test]
#[serial]
async fn can_validation_without_response() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle =
        infra_cfg::server::start_with_route(ctx, "/", post(simple_validation), Some(port)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(get_base_url_port(port))
        .json(&serde_json::json!({"name": "test", "email": "invalid"}))
        .send()
        .await
        .expect("Valid response");

    assert_eq!(res.status(), 400);

    let res_text = res.text().await.expect("response text");
    let res_json: serde_json::Value = serde_json::from_str(&res_text).expect("Valid JSON response");

    let expected_json = serde_json::json!(
        {
            "error": "Bad Request"
        }
    );

    assert_eq!(res_json, expected_json);

    handle.abort();
}
