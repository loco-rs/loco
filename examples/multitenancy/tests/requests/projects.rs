use loco_rs::testing::prelude::*;
use multitenancy::{app::App, models::_entities::users};
use sea_orm::EntityTrait;
use serial_test::serial;

use super::prepare_data;

async fn token_for(ctx: &loco_rs::app::AppContext, id: i64) -> String {
    let user = users::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let jwt = ctx.config.get_jwt_config().unwrap();
    user.generate_jwt(&jwt.secret, jwt.expiration).unwrap()
}

#[tokio::test]
#[serial]
async fn permissions_control_tenant_scoped_project_crud() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let users = prepare_data::init_workspace_users(&ctx).await;
        let owner = token_for(&ctx, users.owner_id).await;
        let support = token_for(&ctx, users.support_id).await;
        let list = request
            .get("/api/tenants/1/projects")
            .authorization_bearer(&support)
            .await;
        assert_eq!(list.status_code(), 200, "{}", list.text());
        assert_eq!(
            list.json::<serde_json::Value>().as_array().unwrap().len(),
            2
        );
        assert_eq!(
            list.json::<serde_json::Value>()[0]["client_name"],
            "Acme Studio"
        );

        let denied = request
            .post("/api/tenants/1/projects")
            .authorization_bearer(&support)
            .json(&serde_json::json!({ "client_id": 1, "name": "Denied Project", "description": "Not allowed" }))
            .await;
        assert_eq!(denied.status_code(), 401);
        let created = request
            .post("/api/tenants/1/projects")
            .authorization_bearer(&owner)
            .json(&serde_json::json!({ "client_id": 1, "name": "New Project", "description": "A tenant project" }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let updated = request
            .put(&format!("/api/tenants/1/projects/{id}"))
            .authorization_bearer(&owner)
            .json(
                &serde_json::json!({ "client_id": 2, "name": "Updated Project", "description": "Updated details" }),
            )
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        assert_eq!(
            updated.json::<serde_json::Value>()["name"],
            "Updated Project"
        );
        assert_eq!(
            updated.json::<serde_json::Value>()["client_name"],
            "Northstar Labs"
        );

        let wrong_client = request
            .post("/api/tenants/1/projects")
            .authorization_bearer(&owner)
            .json(&serde_json::json!({
                "client_id": 3,
                "name": "Cross-tenant client",
                "description": "Must be rejected"
            }))
            .await;
        assert_eq!(wrong_client.status_code(), 404);

        let cross_tenant = request
            .get("/api/tenants/1/projects/3")
            .authorization_bearer(&owner)
            .await;
        assert_eq!(cross_tenant.status_code(), 404);
    })
    .await;
}
