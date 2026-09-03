use loco_rs::prelude::{Set, TenantActiveModelExt};
use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::{_entities::permissions, users},
};
use sea_orm::{ActiveModelTrait, EntityTrait};
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
        assert_eq!(body["stats"]["application_count"], 6);
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
        assert_eq!(members[2]["roles"], serde_json::json!(["Support"]));

        let applications = body["applications"].as_array().unwrap();
        assert_eq!(applications[0]["name"], "Analytics");
        assert_eq!(applications[0]["status"], "inactive");
        assert_eq!(applications[0]["permissions"], serde_json::json!([]));
        assert_eq!(applications[1]["name"], "Billing");
        assert_eq!(applications[2]["name"], "Client Portal");
        assert_eq!(applications[3]["name"], "Documents");
        assert_eq!(applications[1]["permissions"].as_array().unwrap().len(), 2);
        assert_eq!(applications[3]["permissions"].as_array().unwrap().len(), 2);
        assert_eq!(applications[2]["status"], "active");
        assert_eq!(applications[2]["permissions"], serde_json::json!([]));
        assert_eq!(applications[4]["name"], "Feature Flags");
        assert_eq!(applications[4]["status"], "inactive");
        assert_eq!(applications[4]["permissions"], serde_json::json!([]));
        assert_eq!(applications[5]["name"], "Priority Support");
        assert_eq!(applications[5]["status"], "active");
        assert_eq!(applications[5]["permissions"], serde_json::json!([]));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn owner_sees_only_active_workspace_options_and_seeded_addons() {
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
            .get("/api/auth/workspaces")
            .authorization_bearer(&token)
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let workspaces: serde_json::Value = response.json();
        let workspaces = workspaces.as_array().unwrap();
        assert_eq!(workspaces.len(), 2);
        let designer = workspaces
            .iter()
            .find(|workspace| workspace["tenant_name"] == "Designer")
            .unwrap();
        let developer = workspaces
            .iter()
            .find(|workspace| workspace["tenant_name"] == "Developer")
            .unwrap();
        assert_eq!(designer["applications"].as_array().unwrap().len(), 4);
        assert_eq!(developer["applications"].as_array().unwrap().len(), 5);
        let designer_names = designer["applications"]
            .as_array()
            .unwrap()
            .iter()
            .map(|application| application["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        let developer_names = developer["applications"]
            .as_array()
            .unwrap()
            .iter()
            .map(|application| application["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(designer_names.contains(&"Client Portal"));
        assert!(!designer_names.contains(&"Feature Flags"));
        assert!(designer_names.contains(&"Priority Support"));
        assert!(!developer_names.contains(&"Client Portal"));
        assert!(developer_names.contains(&"Feature Flags"));
        assert!(developer_names.contains(&"Priority Support"));
        assert!(developer["applications"]
            .as_array()
            .unwrap()
            .iter()
            .any(|application| application["name"] == "Analytics"));

        let response = request
            .get("/api/tenants/2/dashboard")
            .authorization_bearer(&token)
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let dashboard: serde_json::Value = response.json();
        assert_eq!(dashboard["tenant_name"], "Developer");
        assert_eq!(dashboard["stats"]["member_count"], 1);
        assert_eq!(dashboard["stats"]["application_count"], 6);
        assert_eq!(dashboard["stats"]["document_count"], 1);
        assert_eq!(dashboard["stats"]["invoice_count"], 1);
        assert_eq!(dashboard["applications"][0]["name"], "Analytics");
        assert_eq!(dashboard["applications"][0]["status"], "active");
        assert_eq!(
            dashboard["applications"][0]["permissions"],
            serde_json::json!(["analytics:read"])
        );
        for index in [2, 4, 5] {
            assert_eq!(
                dashboard["applications"][index]["permissions"],
                serde_json::json!([])
            );
        }
        assert_eq!(dashboard["applications"][2]["name"], "Client Portal");
        assert_eq!(dashboard["applications"][2]["status"], "inactive");
        assert_eq!(dashboard["applications"][4]["name"], "Feature Flags");
        assert_eq!(dashboard["applications"][4]["status"], "active");
        assert_eq!(dashboard["applications"][5]["name"], "Priority Support");
        assert_eq!(dashboard["applications"][5]["status"], "active");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn owner_can_change_another_members_role() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let john = users::Entity::find_by_id(1)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let jane = users::Entity::find_by_id(2)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let jwt = ctx.config.get_jwt_config().unwrap();
        let john_token = john.generate_jwt(&jwt.secret, jwt.expiration).unwrap();
        let jane_token = jane.generate_jwt(&jwt.secret, jwt.expiration).unwrap();

        let update = request
            .post("/api/tenants/1/dashboard/members/2/role")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "role": "Support" }))
            .await;
        assert_eq!(update.status_code(), 200, "{}", update.text());
        update.assert_json(&serde_json::json!({
            "member_id": 2,
            "role": "Support"
        }));

        let dashboard = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&john_token)
            .await;
        let body: serde_json::Value = dashboard.json();
        let jane = body["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["member_id"] == 2)
            .unwrap();
        assert_eq!(jane["roles"], serde_json::json!(["Support"]));
        assert_eq!(jane["permissions"].as_array().unwrap().len(), 1);

        let forbidden = request
            .post("/api/tenants/1/dashboard/members/3/role")
            .authorization_bearer(&jane_token)
            .json(&serde_json::json!({ "role": "Manager" }))
            .await;
        assert_eq!(forbidden.status_code(), 401);

        let self_update = request
            .post("/api/tenants/1/dashboard/members/1/role")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "role": "Manager" }))
            .await;
        assert_eq!(self_update.status_code(), 400);

        let invalid = request
            .post("/api/tenants/1/dashboard/members/2/role")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "role": "Superuser" }))
            .await;
        assert_eq!(invalid.status_code(), 400);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn owner_can_assign_workspace_permissions_to_a_role() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();
        let inactive_permission = permissions::ActiveModel {
            tenant_application_id: Set(3),
            key: Set("analytics:read".to_owned()),
            ..Default::default()
        }
        .set_tenant(1)
        .unwrap()
        .insert(&ctx.db)
        .await
        .unwrap();
        let john = users::Entity::find_by_id(1)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let jane = users::Entity::find_by_id(2)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        let jwt = ctx.config.get_jwt_config().unwrap();
        let john_token = john.generate_jwt(&jwt.secret, jwt.expiration).unwrap();
        let jane_token = jane.generate_jwt(&jwt.secret, jwt.expiration).unwrap();

        let dashboard = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&john_token)
            .await;
        assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
        let body: serde_json::Value = dashboard.json();
        assert_eq!(body["roles"].as_array().unwrap().len(), 4);
        assert_eq!(body["roles"][2]["name"], "Manager");
        assert_eq!(body["roles"][2]["permissions"].as_array().unwrap().len(), 3);
        assert_eq!(body["available_permissions"].as_array().unwrap().len(), 4);

        let update = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "permission_ids": [1, 4, 4] }))
            .await;
        assert_eq!(update.status_code(), 200, "{}", update.text());
        update.assert_json(&serde_json::json!({
            "role_id": 3,
            "permission_ids": [1, 4]
        }));

        let dashboard = request
            .get("/api/tenants/1/dashboard")
            .authorization_bearer(&john_token)
            .await;
        let body: serde_json::Value = dashboard.json();
        let jane = body["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["member_id"] == 2)
            .unwrap();
        let keys = jane["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|permission| permission["key"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(keys, ["billing:manage", "documents:read"]);

        let forbidden = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&jane_token)
            .json(&serde_json::json!({ "permission_ids": [1] }))
            .await;
        assert_eq!(forbidden.status_code(), 401);

        let other_workspace_role = request
            .post("/api/tenants/1/dashboard/roles/5/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "permission_ids": [1] }))
            .await;
        assert_eq!(other_workspace_role.status_code(), 404);

        let other_workspace_permission = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "permission_ids": [5] }))
            .await;
        assert_eq!(other_workspace_permission.status_code(), 400);

        let inactive_application_permission = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({
                "permission_ids": [inactive_permission.id]
            }))
            .await;
        assert_eq!(inactive_application_permission.status_code(), 400);

        let too_many_permissions = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "permission_ids": vec![1; 101] }))
            .await;
        assert_eq!(too_many_permissions.status_code(), 400);

        let clear_permissions = request
            .post("/api/tenants/1/dashboard/roles/3/permissions")
            .authorization_bearer(&john_token)
            .json(&serde_json::json!({ "permission_ids": [] }))
            .await;
        assert_eq!(clear_permissions.status_code(), 200);
        clear_permissions.assert_json(&serde_json::json!({
            "role_id": 3,
            "permission_ids": []
        }));
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
