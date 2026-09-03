#![allow(clippy::future_not_send)]

use insta::{assert_debug_snapshot, with_settings};
use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::{_entities::tenants as tenant_entity, roles, users},
};
use rstest::rstest;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serial_test::serial;

use super::prepare_data;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("auth_request");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": "12341234"
        });

        let response = request.post("/api/auth/register").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Register request should succeed"
        );
        let saved_user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("registration should have persisted a user");

        // Snapshot only the fields this test is about, never the whole
        // `Model` — see the note in `tests/models/users.rs`. Anything the
        // snapshot does not cover, assert directly:
        assert!(
            saved_user.email_verification_token.is_some(),
            "registration should issue an email verification token"
        );
        assert!(
            saved_user.email_verified_at.is_none(),
            "a freshly registered user is not verified yet"
        );
        assert_debug_snapshot!((saved_user.email, saved_user.name));

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

        // with_settings!({
        //     filters => cleanup_email()
        // }, {
        //     assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        // });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_register_an_account_without_a_workspace() {
    request::<App, _, _>(|request, ctx| async move {
        let password = "correct-horse-battery-staple";
        let registration = request
            .post("/api/auth/register-account")
            .json(&serde_json::json!({
                "name": "Ada Owner",
                "email": "ada@acme.test",
                "password": password
            }))
            .await;

        assert_eq!(registration.status_code(), 200, "{}", registration.text());
        let session: serde_json::Value = registration.json();
        let token = session["token"].as_str().unwrap();
        assert_eq!(session["name"], "Ada Owner");
        assert!(users::Model::find_by_email(&ctx.db, "ada@acme.test")
            .await
            .is_ok());

        let workspaces = request
            .get("/api/auth/workspaces")
            .authorization_bearer(token)
            .await;
        assert_eq!(workspaces.status_code(), 200, "{}", workspaces.text());
        workspaces.assert_json(&serde_json::json!([]));

        let login = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": "ada@acme.test",
                "password": password
            }))
            .await;
        assert_eq!(login.status_code(), 200, "{}", login.text());
        let login_token = login.json::<serde_json::Value>()["token"]
            .as_str()
            .unwrap()
            .to_owned();
        let after_login = request
            .get("/api/auth/workspaces")
            .authorization_bearer(&login_token)
            .await;
        assert_eq!(after_login.status_code(), 200, "{}", after_login.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn authenticated_user_can_create_another_workspace() {
    request::<App, _, _>(|request, ctx| async move {
        let registration = request
            .post("/api/auth/register-account")
            .json(&serde_json::json!({
                "name": "Ada Owner",
                "email": "ada-two@acme.test",
                "password": "correct-horse-battery-staple"
            }))
            .await;
        assert_eq!(registration.status_code(), 200, "{}", registration.text());
        let token = registration.json::<serde_json::Value>()["token"]
            .as_str()
            .unwrap()
            .to_owned();

        let first = request
            .post("/api/auth/workspaces")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "tenant_name": "Acme Labs",
                "tenant_slug": "acme-labs"
            }))
            .await;
        assert_eq!(first.status_code(), 200, "{}", first.text());

        let created = request
            .post("/api/auth/workspaces")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "tenant_name": "Research Team",
                "tenant_slug": "research-team"
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let workspace = created.json::<serde_json::Value>();
        assert_eq!(workspace["tenant_name"], "Research Team");
        assert_eq!(workspace["tenant_slug"], "research-team");
        assert_eq!(workspace["applications"].as_array().unwrap().len(), 2);
        assert_eq!(workspace["applications"][0]["name"], "Documents");

        let workspaces = request
            .get("/api/auth/workspaces")
            .authorization_bearer(&token)
            .await;
        assert_eq!(workspaces.status_code(), 200, "{}", workspaces.text());
        assert_eq!(
            workspaces
                .json::<serde_json::Value>()
                .as_array()
                .unwrap()
                .len(),
            2
        );

        let tenant_id = workspace["tenant_id"].as_i64().unwrap();
        let application_id = workspace["applications"][0]["id"].as_i64().unwrap();
        let role_names: Vec<String> = roles::Entity::find()
            .filter(roles::Column::TenantId.eq(tenant_id))
            .order_by_asc(roles::Column::Id)
            .all(&ctx.db)
            .await
            .unwrap()
            .into_iter()
            .map(|role| role.name)
            .collect();
        assert_eq!(role_names, ["Owner", "Administrator", "Manager", "Support"]);

        let dashboard = request
            .get(&format!("/api/tenants/{tenant_id}/dashboard"))
            .authorization_bearer(&token)
            .await;
        assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
        let dashboard: serde_json::Value = dashboard.json();
        assert_eq!(dashboard["applications"].as_array().unwrap().len(), 6);
        for addon in dashboard["applications"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|application| {
                !matches!(application["name"].as_str(), Some("Documents" | "Billing"))
            })
        {
            assert_eq!(addon["status"], "inactive");
            assert_eq!(addon["permissions"], serde_json::json!([]));
        }

        let document = request
            .post(&format!(
                "/api/tenants/{tenant_id}/applications/{application_id}/documents"
            ))
            .authorization_bearer(&token)
            .json(&serde_json::json!({ "title": "Research roadmap" }))
            .await;
        assert_eq!(document.status_code(), 200, "{}", document.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn workspace_creation_requires_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/auth/workspaces")
            .json(&serde_json::json!({
                "tenant_name": "Private Team",
                "tenant_slug": "private-team"
            }))
            .await;

        assert_eq!(response.status_code(), 401, "{}", response.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn workspace_creation_rejects_a_duplicate_slug() {
    request::<App, _, _>(|request, ctx| async move {
        let registration = request
            .post("/api/auth/register-account")
            .json(&serde_json::json!({
                "name": "First Owner",
                "email": "first@example.test",
                "password": "correct-horse-battery-staple"
            }))
            .await;
        let token = registration.json::<serde_json::Value>()["token"]
            .as_str()
            .unwrap()
            .to_owned();

        let workspace = serde_json::json!({
            "tenant_name": "Shared Name",
            "tenant_slug": "shared-name"
        });
        let first = request
            .post("/api/auth/workspaces")
            .authorization_bearer(&token)
            .json(&workspace)
            .await;
        assert_eq!(first.status_code(), 200, "{}", first.text());

        let duplicate = request
            .post("/api/auth/workspaces")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "tenant_name": "Other Name",
                "tenant_slug": "shared-name"
            }))
            .await;
        assert_eq!(duplicate.status_code(), 409, "{}", duplicate.text());
        assert_eq!(
            tenant_entity::Entity::find()
                .filter(tenant_entity::Column::Slug.eq("shared-name"))
                .count(&ctx.db)
                .await
                .unwrap(),
            1
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn workspace_creation_rejects_an_invalid_slug() {
    request::<App, _, _>(|request, ctx| async move {
        let registration = request
            .post("/api/auth/register-account")
            .json(&serde_json::json!({
                "name": "Invalid Slug",
                "email": "invalid-slug@example.test",
                "password": "correct-horse-battery-staple"
            }))
            .await;
        let token = registration.json::<serde_json::Value>()["token"]
            .as_str()
            .unwrap()
            .to_owned();

        let response = request
            .post("/api/auth/workspaces")
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "tenant_name": "Invalid Slug Tenant",
                "tenant_slug": "Invalid Slug"
            }))
            .await;

        assert_eq!(response.status_code(), 400, "{}", response.text());
        assert_eq!(
            tenant_entity::Entity::find().count(&ctx.db).await.unwrap(),
            0
        );
    })
    .await;
}

#[rstest]
#[case("login_with_valid_password", "12341234")]
#[case("login_with_invalid_password", "invalid-password")]
#[tokio::test]
#[serial]
async fn can_login_with_verify(#[case] test_name: &str, #[case] password: &str) {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let register_payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": "12341234"
        });

        //Creating a new user
        let register_response = request
            .post("/api/auth/register")
            .json(&register_payload)
            .await;

        assert_eq!(
            register_response.status_code(),
            200,
            "Register request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();
        let email_verification_token = user
            .email_verification_token
            .expect("Email verification token should be generated");
        request
            .get(&format!("/api/auth/verify/{email_verification_token}"))
            .await;

        //verify user request
        let response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        // Make sure email_verified_at is set
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("Failed to find user by email");

        assert!(
            user.email_verified_at.is_some(),
            "Expected the email to be verified, but it was not. User: {user:?}"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(test_name, (response.status_code(), response.text()));
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn login_with_un_existing_email() {
    configure_insta!();

    request::<App, _, _>(|request, _ctx| async move {

        let login_response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": "un_existing@loco.rs",
                "password":  "1234"
            }))
            .await;

        assert_eq!(login_response.status_code(), 401, "Login request should return 401");
        login_response.assert_json(&serde_json::json!({"error": "unauthorized", "description": "You do not have permission to access this resource"}));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_login_without_verify() {
    configure_insta!();

    request::<App, _, _>(|request, _ctx| async move {
        let email = "test@loco.com";
        let password = "12341234";
        let register_payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": password
        });

        //Creating a new user
        let register_response = request
            .post("/api/auth/register")
            .json(&register_payload)
            .await;

        assert_eq!(
            register_response.status_code(),
            200,
            "Register request should succeed"
        );

        //verify user request
        let login_response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        assert_eq!(
            login_response.status_code(),
            200,
            "Login request should succeed"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(login_response.text());
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn invalid_verification_token() {
    configure_insta!();

    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auth/verify/invalid-token").await;

        assert_eq!(response.status_code(), 401, "Verify request should reject");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_reset_password() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let login_data = prepare_data::init_user_login(&request, &ctx).await;

        let forgot_payload = serde_json::json!({
            "email": login_data.user.email,
        });
        let forget_response = request.post("/api/auth/forgot").json(&forgot_payload).await;
        assert_eq!(
            forget_response.status_code(),
            200,
            "Forget request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, &login_data.user.email)
            .await
            .expect("Failed to find user by email");

        assert!(
            user.reset_token.is_some(),
            "Expected reset_token to be set, but it was None. User: {user:?}"
        );
        assert!(
            user.reset_sent_at.is_some(),
            "Expected reset_sent_at to be set, but it was None. User: {user:?}"
        );

        let new_password = "new-password";
        let reset_payload = serde_json::json!({
            "token": user.reset_token,
            "password": new_password,
        });

        let reset_response = request.post("/api/auth/reset").json(&reset_payload).await;
        assert_eq!(
            reset_response.status_code(),
            200,
            "Reset password request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, &user.email)
            .await
            .unwrap();

        assert!(user.reset_token.is_none());
        assert!(user.reset_sent_at.is_none());

        assert_debug_snapshot!(reset_response.text());

        let login_response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": user.email,
                "password": new_password
            }))
            .await;

        assert_eq!(
            login_response.status_code(),
            200,
            "Login request should succeed"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 2, "Exactly one email should be sent");
        // with_settings!({
        //     filters => cleanup_email()
        // }, {
        //     assert_debug_snapshot!(deliveries.messages);
        // });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_current_user() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/auth/current")
            .add_header(auth_key, auth_value)
            .await;

        assert_eq!(
            response.status_code(),
            200,
            "Current request should succeed"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!((response.status_code(), response.text()));
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_auth_with_magic_link() {
    configure_insta!();
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({
            "email": "john@example.com",
        });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Magic link request should succeed"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

        // let redact_token = format!("[a-zA-Z0-9]{{{}}}", users::MAGIC_LINK_LENGTH);
        // with_settings!({
        //      filters => {
        //          let mut combined_filters = cleanup_email().clone();
        //         combined_filters.extend(vec![(r"(\\r\\n|=\\r\\n)", ""), (redact_token.as_str(), "[REDACT_TOKEN]") ]);
        //         combined_filters
        //     }
        // }, {
        //     assert_debug_snapshot!(deliveries.messages);
        // });

        let user = users::Model::find_by_email(&ctx.db, "john@example.com")
            .await
            .expect("User should be found");

        let magic_link_token = user
            .magic_link_token
            .expect("Magic link token should be generated");
        let magic_link_response = request
            .get(&format!("/api/auth/magic-link/{magic_link_token}"))
            .await;
        assert_eq!(
            magic_link_response.status_code(),
            200,
            "Magic link authentication should succeed"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(magic_link_response.text());
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_reject_invalid_email() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let invalid_email = "user1@temp-mail.com";
        let payload = serde_json::json!({
            "email": invalid_email,
        });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            400,
            "Expected request with invalid email '{invalid_email}' to be blocked, but it was allowed."
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_reject_invalid_magic_link_token() {
    configure_insta!();
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let magic_link_response = request.get("/api/auth/magic-link/invalid-token").await;
        assert_eq!(
            magic_link_response.status_code(),
            401,
            "Magic link authentication should be rejected"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_resend_verification_email() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": "12341234"
        });

        let response = request.post("/api/auth/register").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Register request should succeed"
        );

        let resend_payload = serde_json::json!({ "email": email });

        let resend_response = request
            .post("/api/auth/resend-verification-mail")
            .json(&resend_payload)
            .await;

        assert_eq!(
            resend_response.status_code(),
            200,
            "Resend verification email should succeed"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();

        assert_eq!(
            deliveries.count, 2,
            "Two emails should have been sent: welcome and re-verification"
        );

        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("User should exist");

        // Narrowed on purpose — see the note in `tests/models/users.rs`.
        assert!(
            user.email_verification_token.is_some(),
            "resending should leave a verification token on the user"
        );
        assert_debug_snapshot!("resend_verification_user", (user.email, user.name));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_resend_email_if_already_verified() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "verified@loco.com";
        let payload = serde_json::json!({
            "name": "verified",
            "email": email,
            "password": "12341234"
        });

        request.post("/api/auth/register").json(&payload).await;

        // Verify user
        let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();
        if let Some(token) = user.email_verification_token.clone() {
            request.get(&format!("/api/auth/verify/{token}")).await;
        }

        // Try resending verification email
        let resend_payload = serde_json::json!({ "email": email });

        let resend_response = request
            .post("/api/auth/resend-verification-mail")
            .json(&resend_payload)
            .await;

        assert_eq!(
            resend_response.status_code(),
            200,
            "Should return 200 even if already verified"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(
            deliveries.count, 1,
            "Only the original welcome email should be sent"
        );
    })
    .await;
}
