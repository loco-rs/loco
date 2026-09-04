use axum::http::{HeaderName, HeaderValue};
use loco_rs::{app::AppContext, prelude::*, TestServer};
use multitenancy::{
    models::{
        _entities::{tenant_member_roles, tenant_members},
        users,
    },
    views::auth::LoginResponse,
};

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

    add_member(ctx, 1, owner.id, 1).await;
    let support_member = add_member(ctx, 1, support.id, 4).await;
    add_member(ctx, 2, owner.id, 5).await;

    WorkspaceUsers {
        owner_id: owner.id,
        support_id: support.id,
        support_member_id: support_member.id,
    }
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
