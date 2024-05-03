use blo::{
    app::App,
    models::{roles, sea_orm_active_enums, users, users::RegisterParams, users_roles},
};
use loco_rs::{db::truncate_table, prelude::*, testing};
use sea_orm::{ColumnTrait, DatabaseConnection};
use serial_test::serial;
async fn truncate_this(db: &DatabaseConnection) -> Result<(), ModelError> {
    truncate_table(db, users_roles::Entity).await?;
    truncate_table(db, users::Entity).await?;
    truncate_table(db, roles::Entity).await?;
    Ok(()).map_err(|_: ModelError| ModelError::EntityNotFound)
}
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("users_roles");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_connect_user_to_user_role() {
    configure_insta!();

    let boot = testing::boot_test::<App>().await.unwrap();
    testing::seed::<App>(&boot.app_context.db).await.unwrap();
    let _t = truncate_this(&boot.app_context.db).await;
    let new_user: Result<users::Model, ModelError> = users::Model::create_with_password(
        &boot.app_context.db,
        &RegisterParams {
            email: "user1@example.com".to_string(),
            password: "1234".to_string(),
            name: "framework".to_string(),
        },
    )
    .await;
    let new_user = new_user.unwrap();
    // create role
    let role =
        roles::Model::upsert_by_name(&boot.app_context.db, sea_orm_active_enums::RolesName::User)
            .await
            .unwrap();
    // connect user to role
    let user_role =
        users_roles::Model::connect_user_to_role(&boot.app_context.db, &new_user, &role)
            .await
            .unwrap();
    assert_eq!(user_role.users_id, new_user.id);
    assert_eq!(user_role.roles_id, role.id);

    // Find the user role if it exists by user id
    let user_role = users_roles::Entity::find()
        .filter(users_roles::Column::UsersId.eq(new_user.id.clone()))
        .filter(users_roles::Column::RolesId.eq(role.id.clone()))
        .one(&boot.app_context.db)
        .await
        .unwrap();
    assert!(user_role.is_some());
    let user_role = user_role.unwrap();
    assert_eq!(user_role.users_id, new_user.id);
    assert_eq!(user_role.roles_id, role.id);
}

#[tokio::test]
#[serial]
async fn can_connect_user_to_admin_role() {
    configure_insta!();

    let boot = testing::boot_test::<App>().await.unwrap();
    testing::seed::<App>(&boot.app_context.db).await.unwrap();
    let _t = truncate_this(&boot.app_context.db).await;
    let new_user: Result<users::Model, ModelError> = users::Model::create_with_password(
        &boot.app_context.db,
        &RegisterParams {
            email: "user1@example.com".to_string(),
            password: "1234".to_string(),
            name: "framework".to_string(),
        },
    )
    .await;
    let new_user = new_user.unwrap();
    // create role
    let role =
        roles::Model::upsert_by_name(&boot.app_context.db, sea_orm_active_enums::RolesName::Admin)
            .await
            .unwrap();
    // connect user to role
    let user_role =
        users_roles::Model::connect_user_to_role(&boot.app_context.db, &new_user, &role)
            .await
            .unwrap();
    assert_eq!(user_role.users_id, new_user.id);
    assert_eq!(user_role.roles_id, role.id);

    // Find the user role if it exists by user id
    let user_role = users_roles::Entity::find()
        .filter(users_roles::Column::UsersId.eq(new_user.id.clone()))
        .filter(users_roles::Column::RolesId.eq(role.id.clone()))
        .one(&boot.app_context.db)
        .await
        .unwrap();
    assert!(user_role.is_some());
}
