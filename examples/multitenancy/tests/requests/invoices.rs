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
async fn fake_addon_purchase_activates_subscription_and_generates_invoice() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 1).await;

        let purchase = request
            .post("/api/tenants/1/addons/1/purchase")
            .authorization_bearer(&token)
            .json(&serde_json::json!({}))
            .await;
        assert_eq!(purchase.status_code(), 200, "{}", purchase.text());
        let invoice: serde_json::Value = purchase.json();
        assert_eq!(invoice["tenant_id"], 1);
        assert_eq!(invoice["description"], "Analytics add-on purchase");
        assert_eq!(invoice["amount_cents"], 4_900);
        assert_eq!(invoice["status"], "paid");

        let invoices = request
            .get("/api/tenants/1/invoices")
            .authorization_bearer(&token)
            .await;
        assert_eq!(invoices.status_code(), 200, "{}", invoices.text());
        assert_eq!(
            invoices
                .json::<serde_json::Value>()
                .as_array()
                .unwrap()
                .len(),
            3
        );

        let dashboard = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&token)
            .await;
        let dashboard: serde_json::Value = dashboard.json();
        let analytics = dashboard["addons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|addon| addon["name"] == "Analytics")
            .unwrap();
        assert_eq!(analytics["status"], "active");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn invoices_cannot_be_created_directly_or_for_an_active_addon() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 1).await;

        let direct = request
            .post("/api/tenants/1/invoices")
            .authorization_bearer(&token)
            .json(&serde_json::json!({ "number": "MANUAL" }))
            .await;
        assert_eq!(direct.status_code(), 405);

        let duplicate = request
            .post("/api/tenants/1/addons/2/purchase")
            .authorization_bearer(&token)
            .json(&serde_json::json!({}))
            .await;
        assert_eq!(duplicate.status_code(), 400);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn manager_can_read_invoices_but_cannot_purchase_addons() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 2).await;

        let list = request
            .get("/api/tenants/1/invoices")
            .authorization_bearer(&token)
            .await;
        assert_eq!(list.status_code(), 200, "{}", list.text());

        let purchase = request
            .post("/api/tenants/1/addons/3/purchase")
            .authorization_bearer(&token)
            .json(&serde_json::json!({}))
            .await;
        assert_eq!(purchase.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn support_cannot_read_invoices() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 3).await;
        let response = request
            .get("/api/tenants/1/invoices")
            .authorization_bearer(&token)
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
