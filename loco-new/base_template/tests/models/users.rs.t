use chrono::{offset::Local, Duration};
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;
use {{settings.module_name}}::{
    app::App,
    models::users::{self, Model, RegisterParams},
};
use sea_orm::{ActiveModelTrait, ActiveValue, IntoActiveModel};
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("users");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn test_can_validate_model() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");

    let invalid_user = users::ActiveModel {
        name: ActiveValue::set("1".to_string()),
        email: ActiveValue::set("invalid-email".to_string()),
        ..Default::default()
    };

    let res = invalid_user.insert(&boot.app_context.db).await;

    assert_debug_snapshot!(res);
}

#[tokio::test]
#[serial]
async fn can_create_with_password() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");

    let params = RegisterParams {
        email: "test@framework.com".to_string(),
        password: "1234".to_string(),
        name: "framework".to_string(),
    };

    let res = Model::create_with_password(&boot.app_context.db, &params).await;

    insta::with_settings!({
        filters => cleanup_user_model()
    }, {
        assert_debug_snapshot!(res);
    });
}
#[tokio::test]
#[serial]
async fn handle_create_with_password_with_duplicate() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let new_user = Model::create_with_password(
        &boot.app_context.db,
        &RegisterParams {
            email: "user1@example.com".to_string(),
            password: "1234".to_string(),
            name: "framework".to_string(),
        },
    )
    .await;

    assert_debug_snapshot!(new_user);
}

#[tokio::test]
#[serial]
async fn can_find_by_email() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let existing_user = Model::find_by_email(&boot.app_context.db, "user1@example.com").await;
    let non_existing_user_results = Model::find_by_email(&boot.app_context.db, "un@existing-email.com").await;

    assert_debug_snapshot!(existing_user);
    assert_debug_snapshot!(non_existing_user_results);
}

#[tokio::test]
#[serial]
async fn can_find_by_pid() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let existing_user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111").await;
    let non_existing_user_results = Model::find_by_pid(&boot.app_context.db, "23232323-2323-2323-2323-232323232323").await;

    assert_debug_snapshot!(existing_user);
    assert_debug_snapshot!(non_existing_user_results);
}

#[tokio::test]
#[serial]
async fn can_verification_token() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(user.email_verification_sent_at.is_none(), "Expected no email verification sent timestamp");
    assert!(user.email_verification_token.is_none(), "Expected no email verification token");

    let result = user
        .into_active_model()
        .set_email_verification_sent(&boot.app_context.db)
        .await;

    assert!(result.is_ok(), "Failed to set email verification sent");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after setting verification sent");

    assert!(user.email_verification_sent_at.is_some(), "Expected email verification sent timestamp to be present");
    assert!(user.email_verification_token.is_some(), "Expected email verification token to be present");
}


#[tokio::test]
#[serial]
async fn can_set_forgot_password_sent() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(user.reset_sent_at.is_none(), "Expected no reset sent timestamp");
    assert!(user.reset_token.is_none(), "Expected no reset token");

    let result = user
        .into_active_model()
        .set_forgot_password_sent(&boot.app_context.db)
        .await;

    assert!(result.is_ok(), "Failed to set forgot password sent");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after setting forgot password sent");

    assert!(user.reset_sent_at.is_some(), "Expected reset sent timestamp to be present");
    assert!(user.reset_token.is_some(), "Expected reset token to be present");
}


#[tokio::test]
#[serial]
async fn can_verified() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(user.email_verified_at.is_none(), "Expected email to be unverified");

    let result = user
        .into_active_model()
        .verified(&boot.app_context.db)
        .await;

    assert!(result.is_ok(), "Failed to mark email as verified");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after verification");

    assert!(user.email_verified_at.is_some(), "Expected email to be verified");
}


#[tokio::test]
#[serial]
async fn can_reset_password() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(user.verify_password("12341234"), "Password verification failed for original password");

    let result = user
        .clone()
        .into_active_model()
        .reset_password(&boot.app_context.db, "new-password")
        .await;

    assert!(result.is_ok(), "Failed to reset password");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after password reset");

    assert!(user.verify_password("new-password"), "Password verification failed for new password");
}

#[tokio::test]
#[serial]
async fn can_update_user_data() {
    configure_insta!();

    let boot = boot_test::<App>().await.expect("Failed to boot test application");
    seed::<App>(&boot.app_context).await.expect("Failed to seed database");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    let update_params = RegisterParams {
        name: "new-name".to_string(),
        email: "new-email@example.com".to_string(),
        password: "new-password".to_string(),
    };

    let result = user
        .clone()
        .into_active_model()
        .update_user_data(&boot.app_context.db, update_params)
        .await;

    assert!(result.is_ok(), "Failed to update user data");

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after update");



    assert_eq!(user.name, "new-name", "Expected name to be updated");
    assert_eq!(user.email, "new-email@example.com", "Expected email to be updated");
    assert!(user.verify_password("new-password"), "Password verification failed for new password");
}

#[tokio::test]
#[serial]
async fn magic_link() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let user = Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
        .await
        .unwrap();

    assert!(
        user.magic_link_token.is_none(),
        "Magic link token should be initially unset"
    );
    assert!(
        user.magic_link_expiration.is_none(),
        "Magic link expiration should be initially unset"
    );

    let create_result = user
        .into_active_model()
        .create_magic_link(&boot.app_context.db)
        .await;

    assert!(
        create_result.is_ok(),
        "Failed to create magic link: {:?}",
        create_result.unwrap_err()
    );

    let updated_user =
        Model::find_by_pid(&boot.app_context.db, "11111111-1111-1111-1111-111111111111")
            .await
            .expect("Failed to refetch user after magic link creation");

    assert!(
        updated_user.magic_link_token.is_some(),
        "Magic link token should be set after creation"
    );

    let magic_link_token = updated_user.magic_link_token.unwrap();
    assert_eq!(
        magic_link_token.len(),
        users::MAGIC_LINK_LENGTH as usize,
        "Magic link token length does not match expected length"
    );

    assert!(
        updated_user.magic_link_expiration.is_some(),
        "Magic link expiration should be set after creation"
    );

    let now = Local::now();
    let should_expired_at = now + Duration::minutes(users::MAGIC_LINK_EXPIRATION_MIN.into());
    let actual_expiration = updated_user.magic_link_expiration.unwrap();

    assert!(
        actual_expiration >= now,
        "Magic link expiration should be in the future or now"
    );

    assert!(
        actual_expiration <= should_expired_at,
        "Magic link expiration exceeds expected maximum expiration time"
    );
}