use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::_entities::{
        applications, documents, invoices, permissions, role_permissions, roles,
        tenant_applications, tenant_member_roles, tenant_members, tenants, users,
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

    let application_names = applications::Entity::find()
        .order_by_asc(applications::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|application| application.name)
        .collect::<Vec<_>>();
    assert_eq!(application_names, ["Documents", "Billing"]);

    let subscriptions = tenant_applications::Entity::find()
        .order_by_asc(tenant_applications::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(subscriptions.len(), 2);
    assert!(subscriptions
        .iter()
        .all(|subscription| subscription.tenant_id == 1 && subscription.status == "active"));

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

    let members = tenant_members::Entity::find()
        .order_by_asc(tenant_members::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(members.len(), 3);

    let assigned_roles = tenant_member_roles::Entity::find()
        .order_by_asc(tenant_member_roles::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|assignment| (assignment.tenant_member_id, assignment.role_id))
        .collect::<Vec<_>>();
    assert_eq!(assigned_roles, [(1, 1), (2, 2), (3, 3)]);

    let permission_keys: Vec<String> = permissions::Entity::find()
        .order_by_asc(permissions::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|permission| permission.key)
        .collect();
    assert_eq!(
        permission_keys,
        [
            "documents:read",
            "documents:create",
            "billing:read",
            "billing:manage"
        ]
    );

    let grants: Vec<(i64, i64)> = role_permissions::Entity::find()
        .order_by_asc(role_permissions::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|grant| (grant.role_id, grant.permission_id))
        .collect();
    assert_eq!(
        grants,
        [
            (1, 1),
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 1),
            (2, 2),
            (2, 3),
            (3, 1)
        ]
    );

    let document = documents::Entity::find_by_id(1)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (document.tenant_id, document.title.as_str()),
        (1, "Designer onboarding")
    );

    let seeded_invoices = invoices::Entity::find()
        .order_by_asc(invoices::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(seeded_invoices.len(), 2);
    assert_eq!(seeded_invoices[0].number, "INV-1001");
    assert_eq!(seeded_invoices[1].status, "pending");
}
