use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::_entities::{
        applications, clients, documents, permissions, projects, role_permissions, roles,
        tenant_applications, tenant_member_roles, tenant_members, tenants, users,
    },
};
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn seed_creates_only_the_global_addon_catalog() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();
    let db = &boot.app_context.db;

    let application_names = applications::Entity::find()
        .order_by_asc(applications::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|application| application.name)
        .collect::<Vec<_>>();
    assert_eq!(
        application_names,
        [
            "Analytics",
            "Approval Workflows",
            "Feature Flags",
            "Priority Support"
        ]
    );

    assert_eq!(users::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(tenants::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(tenant_members::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(
        tenant_member_roles::Entity::find().count(db).await.unwrap(),
        0
    );
    assert_eq!(roles::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(permissions::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(role_permissions::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(
        tenant_applications::Entity::find().count(db).await.unwrap(),
        0
    );
    assert_eq!(clients::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(projects::Entity::find().count(db).await.unwrap(), 0);
    assert_eq!(documents::Entity::find().count(db).await.unwrap(), 0);
}
