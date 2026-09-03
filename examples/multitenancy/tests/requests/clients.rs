use loco_rs::testing::prelude::*;
use multitenancy::{app::App, models::_entities::users};
use sea_orm::EntityTrait;
use serial_test::serial;

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
async fn permissions_control_tenant_scoped_client_crud() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let owner = token_for(&ctx, 1).await;
        let support = token_for(&ctx, 3).await;
        let list = request
            .get("/api/tenants/1/clients")
            .authorization_bearer(&support)
            .await;
        assert_eq!(list.status_code(), 200, "{}", list.text());
        assert_eq!(
            list.json::<serde_json::Value>().as_array().unwrap().len(),
            2
        );

        let denied = request
            .post("/api/tenants/1/clients")
            .authorization_bearer(&support)
            .json(&serde_json::json!({ "name": "Denied Client", "email": "denied@example.com" }))
            .await;
        assert_eq!(denied.status_code(), 401);
        let created = request
            .post("/api/tenants/1/clients")
            .authorization_bearer(&owner)
            .json(&serde_json::json!({ "name": "New Client", "email": "new@example.com" }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let updated = request
            .put(&format!("/api/tenants/1/clients/{id}"))
            .authorization_bearer(&owner)
            .json(&serde_json::json!({ "name": "Updated Client", "email": "updated@example.com" }))
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        assert_eq!(
            updated.json::<serde_json::Value>()["name"],
            "Updated Client"
        );

        let cross_tenant = request
            .get("/api/tenants/1/clients/3")
            .authorization_bearer(&owner)
            .await;
        assert_eq!(cross_tenant.status_code(), 404);
    })
    .await;
}
