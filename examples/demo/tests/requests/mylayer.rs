use demo_app::{app::App, views::user::UserResponse};
use loco_rs::{testing, tokio};
use serial_test::serial;

use crate::requests::prepare_data;
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("mylayer_request");
        let _guard = settings.bind_to_scope();
    };
}
#[tokio::test]
#[serial]
async fn cannot_get_echo_when_no_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/mylayer/echo")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_echo_when_admin_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/user/convert/admin")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
        let body = response.json::<UserResponse>();
        assert_eq!(body.role, "Admin");

        let response = request
            .get("/mylayer/echo")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
    })
    .await;
}
#[tokio::test]
#[serial]
async fn can_get_echo_when_user_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/user/convert/user")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
        let body = response.json::<UserResponse>();
        assert_eq!(body.role, "User");

        let response = request
            .get("/mylayer/echo")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_get_admin_when_no_role() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/mylayer/admin")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_get_admin_when_user_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/user/convert/user")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
        let body = response.json::<UserResponse>();
        assert_eq!(body.role, "User");

        let response = request
            .get("/mylayer/admin")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_admin_when_admin_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/user/convert/admin")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
        let body = response.json::<UserResponse>();
        assert_eq!(body.role, "Admin");

        let response = request
            .get("/mylayer/admin")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_get_user_when_no_role() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/mylayer/user")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_user_when_user_role_assigned() {
    configure_insta!();
    testing::request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .post("/user/convert/user")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
        let body = response.json::<UserResponse>();
        assert_eq!(body.role, "User");

        let response = request
            .get("/mylayer/user")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(response.status_code(), 200);
    })
    .await;
}
