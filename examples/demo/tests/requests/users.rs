use axum::http::{HeaderName, HeaderValue};
use blo::{app::App, models::users, views::auth::LoginResponse};
use insta::{assert_debug_snapshot, with_settings};
use migration::Migrator;
use rustyrails::testing;
use serial_test::serial;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    testing::request::<App, Migrator, _, _>(|request, ctx| async move {
        let email = "test@framework.com";
        let payload = serde_json::json!({
            "name": "framework",
            "email": email,
            "password": "12341234"
        });

        let _response = request.post("/auth/register").json(&payload).await;
        let saved_user = users::Model::find_by_email(&ctx.db, email).await;

        with_settings!({
            filters => testing::cleanup_user_model()
        }, {
            assert_debug_snapshot!(saved_user);
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_login() {
    configure_insta!();

    testing::request::<App, Migrator, _, _>(|request, _ctx| async move {
        let email = "test@framework.com";
        let password = "12341234";
        let payload = serde_json::json!({
            "name": "framework",
            "email": email,
            "password": password
        });

        _ = request.post("/auth/register").json(&payload).await;
        let response = request
            .post("/auth/login")
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        with_settings!({
            filters => testing::cleanup_user_model()
        }, {
            assert_debug_snapshot!((response.status_code(), response.text()));
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_handle_invalid_login() {
    configure_insta!();

    testing::request::<App, Migrator, _, _>(|request, _ctx| async move {
        let email = "test@framework.com";
        let payload = serde_json::json!({
            "name": "framework",
            "email": email,
            "password": "12341234"
        });

        _ = request.post("/auth/register").json(&payload).await;
        let res = request
            .post("/auth/login")
            .json(&serde_json::json!({
                "email": email,
                "password": "invalid-password"
            }))
            .await;

        assert_debug_snapshot!(res);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn get_validate_user_login_flow() {
    configure_insta!();
    testing::request::<App, Migrator, _, _>(|request, _ctx| async move {
        let email = "test@framework.com";
        let password = "12341234";
        let register_payload = serde_json::json!({
            "name": "framework",
            "email": email,
            "password": password
        });
        let login_payload = serde_json::json!({
            "email": email,
            "password": password
        });

        _ = request.post("/auth/register").json(&register_payload).await;
        let login_response = request.post("/auth/login").json(&login_payload).await;
        let login_response: LoginResponse = serde_json::from_str(&login_response.text()).unwrap();

        let auth_header_value =
            HeaderValue::from_str(&format!("Bearer {}", &login_response.token)).unwrap();
        let current_user_request = request
            .get("/user/current")
            .add_header(HeaderName::from_static("authorization"), auth_header_value)
            .await;

        with_settings!({
            filters => testing::cleanup_user_model()
        }, {
            assert_debug_snapshot!(current_user_request.text());
        });
    })
    .await;
}
