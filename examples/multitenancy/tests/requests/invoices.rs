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
async fn owner_lists_and_creates_tenant_invoices() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 1).await;
        let url = "/api/tenants/1/applications/2/invoices";

        let list = request.get(url).authorization_bearer(&token).await;
        assert_eq!(list.status_code(), 200, "{}", list.text());
        let invoices: serde_json::Value = list.json();
        assert_eq!(invoices.as_array().unwrap().len(), 2);

        let create = request
            .post(url)
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "number": "INV-1003",
                "amount_cents": 7900,
                "status": "draft"
            }))
            .await;
        assert_eq!(create.status_code(), 200, "{}", create.text());
        let invoice: serde_json::Value = create.json();
        assert_eq!(invoice["tenant_id"], 1);
        assert_eq!(invoice["tenant_application_id"], 2);
        assert_eq!(invoice["number"], "INV-1003");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn manager_can_read_billing_but_cannot_manage_it() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 2).await;
        let url = "/api/tenants/1/applications/2/invoices";

        let list = request.get(url).authorization_bearer(&token).await;
        assert_eq!(list.status_code(), 200, "{}", list.text());

        let create = request
            .post(url)
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "number": "INV-DENIED",
                "amount_cents": 100,
                "status": "draft"
            }))
            .await;
        assert_eq!(create.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn viewer_cannot_read_billing() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 3).await;

        let response = request
            .get("/api/tenants/1/applications/2/invoices")
            .authorization_bearer(&token)
            .await;

        assert_eq!(response.status_code(), 401);
    })
    .await;
}
