use loco_rs::testing::prelude::*;
use multitenancy::{app::App, models::_entities::users};
use sea_orm::EntityTrait;
use serial_test::serial;

async fn token_for(ctx: &loco_rs::app::AppContext, user_id: i64) -> String {
    let user = users::Entity::find_by_id(user_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let jwt = ctx.config.get_jwt_config().unwrap();
    user.generate_jwt(&jwt.secret, jwt.expiration).unwrap()
}

#[tokio::test]
#[serial]
async fn owner_can_list_create_view_and_edit_documents() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 2).await;
        let base = "/api/tenants/1/documents";

        let list = request.get(base).authorization_bearer(&token).await;
        assert_eq!(list.status_code(), 200, "{}", list.text());
        let records: serde_json::Value = list.json();
        assert_eq!(records.as_array().unwrap().len(), 1);
        assert_eq!(records[0]["title"], "Designer onboarding");
        assert_eq!(
            records[0]["description"],
            "Welcome materials and delivery notes for the Designer workspace."
        );

        let create = request
            .post(base)
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "title": "Launch plan",
                "description": "Milestones and responsibilities for launch."
            }))
            .await;
        assert_eq!(create.status_code(), 200, "{}", create.text());
        let id = create.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let update = request
            .put(&format!("{base}/{id}"))
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "title": "Updated launch plan",
                "description": "The revised milestones and launch responsibilities."
            }))
            .await;
        assert_eq!(update.status_code(), 200, "{}", update.text());
        assert_eq!(
            update.json::<serde_json::Value>()["title"],
            "Updated launch plan"
        );

        let show = request
            .get(&format!("{base}/{id}"))
            .authorization_bearer(&token)
            .await;
        assert_eq!(show.status_code(), 200, "{}", show.text());
        assert_eq!(show.json::<serde_json::Value>()["tenant_id"], 1);
        assert_eq!(
            show.json::<serde_json::Value>()["description"],
            "The revised milestones and launch responsibilities."
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn support_can_view_but_cannot_create_or_edit_documents() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 3).await;

        let list = request
            .get("/api/tenants/1/documents")
            .authorization_bearer(&token)
            .await;
        assert_eq!(list.status_code(), 200, "{}", list.text());

        let create = request
            .post("/api/tenants/1/documents")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "title": "Denied",
                "description": "This write should not be permitted."
            }))
            .await;
        assert_eq!(create.status_code(), 401);

        let update = request
            .put("/api/tenants/1/documents/1")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "title": "Denied",
                "description": "This edit should not be permitted."
            }))
            .await;
        assert_eq!(update.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn tenant_scope_prevents_cross_workspace_document_access() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 2).await;

        let response = request
            .get("/api/tenants/1/documents/2")
            .authorization_bearer(&token)
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}
