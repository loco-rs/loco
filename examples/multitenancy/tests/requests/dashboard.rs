use loco_rs::testing::prelude::*;
use multitenancy::{app::App, models::users};
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
async fn owner_sees_seeded_tenant_resources_roles_and_addons() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 1).await;
        let response = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&token)
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: serde_json::Value = response.json();

        assert_eq!(body["tenant_name"], "Designer");
        assert_eq!(body["stats"]["member_count"], 3);
        assert_eq!(body["stats"]["addon_count"], 4);
        assert_eq!(body["stats"]["client_count"], 2);
        assert_eq!(body["stats"]["project_count"], 2);
        assert_eq!(body["stats"]["document_count"], 1);
        assert_eq!(body["stats"]["invoice_count"], 2);
        assert_eq!(
            body["current_member"]["roles"],
            serde_json::json!(["Owner"])
        );
        assert_eq!(body["available_permissions"].as_array().unwrap().len(), 11);
        assert_eq!(body["roles"].as_array().unwrap().len(), 4);

        let addons = body["addons"].as_array().unwrap();
        assert_eq!(addons.len(), 4);
        assert_eq!(addons[0]["name"], "Analytics");
        assert_eq!(addons[0]["status"], "inactive");
        assert_eq!(addons[1]["name"], "Client Portal");
        assert_eq!(addons[1]["status"], "active");
        assert!(addons
            .iter()
            .all(|addon| addon.get("permissions").is_none()));

        let members = body["members"].as_array().unwrap();
        assert_eq!(members.len(), 3);
        assert!(members.iter().any(|member| member["name"] == "Jane Smith"
            && member["roles"] == serde_json::json!(["Manager"])));
        assert!(members.iter().any(|member| member["name"] == "Sam Lee"
            && member["roles"] == serde_json::json!(["Support"])));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn workspace_list_contains_tenants_once_and_developer_has_expected_addons() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let token = token_for(&ctx, 1).await;
        let workspaces = request
            .get("/api/auth/workspaces")
            .authorization_bearer(&token)
            .await;
        let body: serde_json::Value = workspaces.json();
        assert_eq!(body.as_array().unwrap().len(), 2);
        assert!(body
            .as_array()
            .unwrap()
            .iter()
            .all(|workspace| workspace.get("applications").is_none()));

        let dashboard = request
            .get("/api/tenants/2/dashboard")
            .authorization_bearer(&token)
            .await;
        let body: serde_json::Value = dashboard.json();
        let statuses = body["addons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|addon| {
                (
                    addon["name"].as_str().unwrap(),
                    addon["status"].as_str().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            [
                ("Analytics", "active"),
                ("Client Portal", "inactive"),
                ("Feature Flags", "active"),
                ("Priority Support", "active")
            ]
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn owner_can_change_member_role_and_role_permissions() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let owner_token = token_for(&ctx, 1).await;
        let manager_token = token_for(&ctx, 2).await;

        let role_update = request
            .post("/api/tenants/1/dashboard/members/2/role")
            .authorization_bearer(&owner_token)
            .json(&serde_json::json!({ "role": "Support" }))
            .await;
        assert_eq!(role_update.status_code(), 200, "{}", role_update.text());

        let dashboard = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&owner_token)
            .await;
        let body: serde_json::Value = dashboard.json();
        let document_view_id = body["available_permissions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|permission| permission["key"] == "documents:view")
            .unwrap()["id"]
            .as_i64()
            .unwrap();
        let role_update = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&owner_token)
            .json(&serde_json::json!({ "permission_ids": [document_view_id, document_view_id] }))
            .await;
        assert_eq!(role_update.status_code(), 200, "{}", role_update.text());
        assert_eq!(
            role_update.json::<serde_json::Value>()["permission_ids"],
            serde_json::json!([document_view_id])
        );

        let forbidden = request
            .post("/api/tenants/1/dashboard/roles/4/permissions")
            .authorization_bearer(&manager_token)
            .json(&serde_json::json!({ "permission_ids": [] }))
            .await;
        assert_eq!(forbidden.status_code(), 401);
        let wrong_tenant = request
            .post("/api/tenants/1/dashboard/roles/5/permissions")
            .authorization_bearer(&owner_token)
            .json(&serde_json::json!({ "permission_ids": [] }))
            .await;
        assert_eq!(wrong_tenant.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn non_member_cannot_view_dashboard() {
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
