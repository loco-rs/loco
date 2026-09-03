use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::_entities::{
        applications, documents, permissions, role_permissions, roles, tenant_applications,
        tenant_member_roles, tenant_members, tenants, users,
    },
};
use sea_orm::{EntityTrait, QueryOrder};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn seed_creates_the_designer_workspace_and_roles() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();
    let db = &boot.app_context.db;

    let user = users::Entity::find_by_id(1).one(db).await.unwrap().unwrap();
    assert_eq!(user.name, "John Doe");
    assert_eq!(user.email, "john@example.com");
    assert!(user.verify_password("password"));

    let tenant = tenants::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (tenant.name.as_str(), tenant.slug.as_str()),
        ("Designer", "designer")
    );

    let application = applications::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(application.name, "Documents");

    let subscription = tenant_applications::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (subscription.tenant_id, subscription.status.as_str()),
        (1, "active")
    );

    let member = tenant_members::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((member.tenant_id, member.user_id), (1, 1));

    let role_names: Vec<String> = roles::Entity::find()
        .order_by_asc(roles::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|role| role.name)
        .collect();
    assert_eq!(role_names, ["Owner", "Manager", "Viewer"]);

    let assigned_role = tenant_member_roles::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (assigned_role.tenant_member_id, assigned_role.role_id),
        (1, 1)
    );

    let permission_keys: Vec<String> = permissions::Entity::find()
        .order_by_asc(permissions::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|permission| permission.key)
        .collect();
    assert_eq!(permission_keys, ["documents:read", "documents:create"]);

    let grants: Vec<(i64, i64)> = role_permissions::Entity::find()
        .order_by_asc(role_permissions::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|grant| (grant.role_id, grant.permission_id))
        .collect();
    assert_eq!(grants, [(1, 1), (1, 2), (2, 1), (2, 2), (3, 1)]);

    let document = documents::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (document.tenant_id, document.title.as_str()),
        (1, "Designer onboarding")
    );
}
