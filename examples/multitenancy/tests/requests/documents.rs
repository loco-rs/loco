use loco_rs::prelude::{AppContext, TenantActiveModelExt, TenantQueryExt};
use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::{
        applications, documents, permissions, role_permissions, roles, tenant_applications,
        tenant_member_roles, tenant_members, tenants, users,
    },
};
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter,
};
use serial_test::serial;

struct DemoData {
    api_key: String,
    tenant_a_id: i64,
    tenant_b_id: i64,
    documents_app_id: i64,
    billing_app_id: i64,
    billing_subscription_id: i64,
    editor_role_id: i64,
}

async fn setup(ctx: &AppContext) -> DemoData {
    let user = users::Model::create_with_password(
        &ctx.db,
        &users::RegisterParams {
            email: "owner@example.com".to_owned(),
            password: "correct horse battery staple".to_owned(),
            name: "Owner".to_owned(),
        },
    )
    .await
    .unwrap();

    let tenant_a = tenants::ActiveModel {
        name: Set("Acme".to_owned()),
        slug: Set("acme".to_owned()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();
    let tenant_b = tenants::ActiveModel {
        name: Set("Globex".to_owned()),
        slug: Set("globex".to_owned()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();

    let documents_app = applications::ActiveModel {
        name: Set("Documents".to_owned()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();
    let billing_app = applications::ActiveModel {
        name: Set("Billing".to_owned()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .unwrap();

    let acme_documents = tenant_applications::ActiveModel {
        application_id: Set(documents_app.id),
        status: Set("active".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    tenant_applications::ActiveModel {
        application_id: Set(documents_app.id),
        status: Set("active".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_b.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    let billing_subscription = tenant_applications::ActiveModel {
        application_id: Set(billing_app.id),
        status: Set("active".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();

    let member = tenant_members::ActiveModel {
        user_id: Set(user.id),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    let editor = roles::ActiveModel {
        name: Set("Editor".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    tenant_member_roles::ActiveModel {
        tenant_member_id: Set(member.id),
        role_id: Set(editor.id),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();

    for key in ["documents:read", "documents:create"] {
        let permission = permissions::ActiveModel {
            tenant_application_id: Set(acme_documents.id),
            key: Set(key.to_owned()),
            ..Default::default()
        }
        .set_tenant(tenant_a.id)
        .unwrap()
        .insert(&ctx.db)
        .await
        .unwrap();

        role_permissions::ActiveModel {
            role_id: Set(editor.id),
            permission_id: Set(permission.id),
            ..Default::default()
        }
        .set_tenant(tenant_a.id)
        .unwrap()
        .insert(&ctx.db)
        .await
        .unwrap();
    }

    documents::ActiveModel {
        title: Set("Acme roadmap".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_a.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    documents::ActiveModel {
        title: Set("Globex secret".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_b.id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();

    DemoData {
        api_key: user.api_key,
        tenant_a_id: tenant_a.id,
        tenant_b_id: tenant_b.id,
        documents_app_id: documents_app.id,
        billing_app_id: billing_app.id,
        billing_subscription_id: billing_subscription.id,
        editor_role_id: editor.id,
    }
}

fn documents_url(tenant_id: i64, application_id: i64) -> String {
    format!("/api/tenants/{tenant_id}/applications/{application_id}/documents")
}

#[tokio::test]
#[serial]
async fn member_only_sees_documents_from_the_requested_tenant() {
    request::<App, _, _>(|request, ctx| async move {
        let demo = setup(&ctx).await;
        let response = request
            .get(&documents_url(demo.tenant_a_id, demo.documents_app_id))
            .authorization_bearer(&demo.api_key)
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: serde_json::Value = response.json();
        let documents = body.as_array().unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0]["title"], "Acme roadmap");
        assert_eq!(documents[0]["tenant_id"], demo.tenant_a_id);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn membership_and_roles_do_not_cross_tenant_boundaries() {
    request::<App, _, _>(|request, ctx| async move {
        let demo = setup(&ctx).await;
        let response = request
            .get(&documents_url(demo.tenant_b_id, demo.documents_app_id))
            .authorization_bearer(&demo.api_key)
            .await;

        assert_eq!(response.status_code(), 401);
        response.assert_json(&serde_json::json!({
            "error": "unauthorized",
            "description": "You do not have permission to access this resource"
        }));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn permissions_are_limited_to_the_subscribed_application() {
    request::<App, _, _>(|request, ctx| async move {
        let demo = setup(&ctx).await;
        let active_without_permission = request
            .get(&documents_url(demo.tenant_a_id, demo.billing_app_id))
            .authorization_bearer(&demo.api_key)
            .await;

        assert_eq!(active_without_permission.status_code(), 401);

        let permission = permissions::ActiveModel {
            tenant_application_id: Set(demo.billing_subscription_id),
            key: Set("documents:read".to_owned()),
            ..Default::default()
        }
        .set_tenant(demo.tenant_a_id)
        .unwrap()
        .insert(&ctx.db)
        .await
        .unwrap();
        role_permissions::ActiveModel {
            role_id: Set(demo.editor_role_id),
            permission_id: Set(permission.id),
            ..Default::default()
        }
        .set_tenant(demo.tenant_a_id)
        .unwrap()
        .insert(&ctx.db)
        .await
        .unwrap();
        tenant_applications::Entity::update_many()
            .col_expr(tenant_applications::Column::Status, Expr::value("inactive"))
            .filter(tenant_applications::Column::Id.eq(demo.billing_subscription_id))
            .in_tenant(demo.tenant_a_id)
            .exec(&ctx.db)
            .await
            .unwrap();

        let inactive_with_permission = request
            .get(&documents_url(demo.tenant_a_id, demo.billing_app_id))
            .authorization_bearer(&demo.api_key)
            .await;

        assert_eq!(inactive_with_permission.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn member_with_create_permission_creates_in_the_trusted_tenant() {
    request::<App, _, _>(|request, ctx| async move {
        let demo = setup(&ctx).await;
        let response = request
            .post(&documents_url(demo.tenant_a_id, demo.documents_app_id))
            .authorization_bearer(&demo.api_key)
            .json(&serde_json::json!({ "title": "Launch plan" }))
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: serde_json::Value = response.json();
        assert_eq!(body["title"], "Launch plan");
        assert_eq!(body["tenant_id"], demo.tenant_a_id);

        let globex_documents = documents::Entity::find()
            .in_tenant(demo.tenant_b_id)
            .all(&ctx.db)
            .await
            .unwrap();
        assert_eq!(globex_documents.len(), 1);
        assert_eq!(globex_documents[0].title, "Globex secret");
    })
    .await;
}
