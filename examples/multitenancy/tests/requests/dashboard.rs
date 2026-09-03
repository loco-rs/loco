use loco_rs::testing::prelude::*;
use multitenancy::{app::App, models::users};
use sea_orm::EntityTrait;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn owner_sees_seeded_workspace_access_graph() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let john = users::Entity::find_by_id(1)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let jwt = ctx.config.get_jwt_config().unwrap();
        let token = john.generate_jwt(&jwt.secret, jwt.expiration).unwrap();

        let response = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&token)
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: serde_json::Value = response.json();
        assert_eq!(body["tenant_name"], "Designer");
        assert_eq!(body["stats"]["member_count"], 3);
        assert_eq!(body["stats"]["application_count"], 2);
        assert_eq!(body["stats"]["document_count"], 1);
        assert_eq!(body["stats"]["invoice_count"], 2);
        assert_eq!(body["current_member"]["name"], "John Doe");
        assert_eq!(
            body["current_member"]["roles"],
            serde_json::json!(["Owner"])
        );

        let members = body["members"].as_array().unwrap();
        assert_eq!(members.len(), 3);
        assert_eq!(members[0]["name"], "Jane Smith");
        assert_eq!(members[0]["roles"], serde_json::json!(["Manager"]));
        assert_eq!(members[1]["name"], "John Doe");
        assert_eq!(members[2]["roles"], serde_json::json!(["Viewer"]));

        let applications = body["applications"].as_array().unwrap();
        assert_eq!(applications[0]["name"], "Billing");
        assert_eq!(applications[1]["name"], "Documents");
        assert_eq!(applications[0]["permissions"].as_array().unwrap().len(), 2);
        assert_eq!(applications[1]["permissions"].as_array().unwrap().len(), 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn non_member_cannot_view_a_workspace_dashboard() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let outsider = users::Model::create_with_password(
            &ctx.db,
            &users::RegisterParams {
                name: "Outsider".to_owned(),
                email: "outsider@example.com".to_owned(),
                password: "password".to_owned(),
            },
        )
        .await
        .unwrap();
        let jwt = ctx.config.get_jwt_config().unwrap();
        let token = outsider.generate_jwt(&jwt.secret, jwt.expiration).unwrap();

        let response = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&token)
            .await;

        assert_eq!(response.status_code(), 401);
    })
    .await;
}
