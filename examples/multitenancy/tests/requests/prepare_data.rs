use axum::http::{HeaderName, HeaderValue};
use loco_rs::{app::AppContext, prelude::*, TestServer};
use multitenancy::{
    models::{
        _entities::{
            clients, documents, projects, roles, tenant_applications, tenant_member_roles,
            tenant_members,
        },
        tenants, users,
    },
    views::auth::LoginResponse,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

const USER_EMAIL: &str = "test@loco.com";
const USER_PASSWORD: &str = "1234";

pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

pub struct WorkspaceUsers {
    pub owner_id: i64,
    pub support_id: i64,
    pub support_member_id: i64,
}

pub async fn init_workspace_users(ctx: &AppContext) -> WorkspaceUsers {
    let owner = users::Model::create_with_password(
        &ctx.db,
        &users::RegisterParams {
            name: "Test Owner".to_owned(),
            email: "owner@example.com".to_owned(),
            password: "password".to_owned(),
        },
    )
    .await
    .unwrap();
    let support = users::Model::create_with_password(
        &ctx.db,
        &users::RegisterParams {
            name: "Test Support".to_owned(),
            email: "support@example.com".to_owned(),
            password: "password".to_owned(),
        },
    )
    .await
    .unwrap();

    let designer = tenants::Model::create_workspace(&ctx.db, owner.id, "Designer")
        .await
        .unwrap()
        .tenant;
    let developer = tenants::Model::create_workspace(&ctx.db, owner.id, "Developer")
        .await
        .unwrap()
        .tenant;

    assert_eq!(designer.id, 1);
    assert_eq!(developer.id, 2);

    let support_role = roles::Entity::find()
        .filter(roles::Column::TenantId.eq(designer.id))
        .filter(roles::Column::Name.eq("Support"))
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let support_member = add_member(ctx, designer.id, support.id, support_role.id).await;

    let acme = create_client(ctx, designer.id, "Acme Studio", "hello@acme.example").await;
    let northstar =
        create_client(ctx, designer.id, "Northstar Labs", "team@northstar.example").await;
    let community = create_client(
        ctx,
        developer.id,
        "Loco Community",
        "community@loco.example",
    )
    .await;

    create_project(
        ctx,
        designer.id,
        acme.id,
        "Brand refresh",
        "Refresh the visual identity and client-facing design system.",
    )
    .await;
    create_project(
        ctx,
        designer.id,
        northstar.id,
        "Client launch",
        "Coordinate launch assets and delivery milestones.",
    )
    .await;
    create_project(
        ctx,
        developer.id,
        community.id,
        "API platform",
        "Build and document the next version of the developer API.",
    )
    .await;

    create_document(
        ctx,
        designer.id,
        "Designer onboarding",
        "Welcome materials and delivery notes for the Designer workspace.",
    )
    .await;
    create_document(
        ctx,
        developer.id,
        "Developer architecture notes",
        "Technical decisions and architecture notes for the Developer workspace.",
    )
    .await;

    for application_id in [2, 4] {
        activate_addon(ctx, designer.id, application_id).await;
    }
    for application_id in [1, 3, 4] {
        activate_addon(ctx, developer.id, application_id).await;
    }

    WorkspaceUsers {
        owner_id: owner.id,
        support_id: support.id,
        support_member_id: support_member.id,
    }
}

async fn create_client(
    ctx: &AppContext,
    tenant_id: i64,
    name: &str,
    email: &str,
) -> clients::Model {
    clients::ActiveModel {
        name: Set(name.to_owned()),
        email: Set(email.to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap()
}

async fn create_project(
    ctx: &AppContext,
    tenant_id: i64,
    client_id: i64,
    name: &str,
    description: &str,
) -> projects::Model {
    projects::ActiveModel {
        client_id: Set(client_id),
        name: Set(name.to_owned()),
        description: Set(description.to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap()
}

async fn create_document(
    ctx: &AppContext,
    tenant_id: i64,
    title: &str,
    description: &str,
) -> documents::Model {
    documents::ActiveModel {
        title: Set(title.to_owned()),
        description: Set(description.to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap()
}

async fn activate_addon(ctx: &AppContext, tenant_id: i64, application_id: i64) {
    let addon = tenant_applications::Entity::find()
        .filter(tenant_applications::Column::TenantId.eq(tenant_id))
        .filter(tenant_applications::Column::ApplicationId.eq(application_id))
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let mut addon: tenant_applications::ActiveModel = addon.into();
    addon.status = Set("active".to_owned());
    addon.update(&ctx.db).await.unwrap();
}

async fn add_member(
    ctx: &AppContext,
    tenant_id: i64,
    user_id: i64,
    role_id: i64,
) -> tenant_members::Model {
    let member = tenant_members::ActiveModel {
        user_id: Set(user_id),
        ..Default::default()
    }
    .set_tenant(tenant_id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    tenant_member_roles::ActiveModel {
        tenant_member_id: Set(member.id),
        role_id: Set(role_id),
        ..Default::default()
    }
    .set_tenant(tenant_id)
    .unwrap()
    .insert(&ctx.db)
    .await
    .unwrap();
    member
}

#[allow(clippy::future_not_send)]
pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    let register_payload = serde_json::json!({
        "name": "loco",
        "email": USER_EMAIL,
        "password": USER_PASSWORD
    });

    //Creating a new user
    request
        .post("/api/auth/register")
        .json(&register_payload)
        .await;
    let user = users::Model::find_by_email(&ctx.db, USER_EMAIL)
        .await
        .unwrap();

    let verify_payload = serde_json::json!({
        "token": user.email_verification_token,
    });

    request.post("/api/auth/verify").json(&verify_payload).await;

    let response = request
        .post("/api/auth/login")
        .json(&serde_json::json!({
            "email": USER_EMAIL,
            "password": USER_PASSWORD
        }))
        .await;

    let login_response: LoginResponse = serde_json::from_str(&response.text()).unwrap();

    LoggedInUser {
        user: users::Model::find_by_email(&ctx.db, USER_EMAIL)
            .await
            .unwrap(),
        token: login_response.token,
    }
}

pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
