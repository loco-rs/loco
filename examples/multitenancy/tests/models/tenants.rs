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
async fn seed_creates_two_workspaces_with_application_availability() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();
    let db = &boot.app_context.db;

    let user = users::Entity::find_by_id(1).one(db).await.unwrap().unwrap();
    assert_eq!(user.name, "John Doe");
    assert_eq!(user.email, "john@example.com");
    assert!(user.verify_password("password"));

    let seeded_tenants = tenants::Entity::find()
        .order_by_asc(tenants::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(
        seeded_tenants
            .iter()
            .map(|tenant| (tenant.name.as_str(), tenant.slug.as_str()))
            .collect::<Vec<_>>(),
        [("Designer", "designer"), ("Developer", "developer")]
    );

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
            "Documents",
            "Billing",
            "Analytics",
            "Client Portal",
            "Feature Flags",
            "Priority Support"
        ]
    );

    let tenant_application_rows = tenant_applications::Entity::find()
        .order_by_asc(tenant_applications::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(
        tenant_application_rows
            .iter()
            .map(|tenant_application| (
                tenant_application.tenant_id,
                tenant_application.application_id,
                tenant_application.status.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (1, 1, "active"),
            (1, 2, "active"),
            (1, 3, "inactive"),
            (2, 1, "active"),
            (2, 2, "active"),
            (2, 3, "active"),
            (1, 4, "active"),
            (1, 5, "inactive"),
            (1, 6, "active"),
            (2, 4, "inactive"),
            (2, 5, "active"),
            (2, 6, "active")
        ]
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
    assert_eq!(
        role_names,
        [
            "Owner",
            "Administrator",
            "Manager",
            "Support",
            "Owner",
            "Administrator",
            "Manager",
            "Support"
        ]
    );

    let members = tenant_members::Entity::find()
        .order_by_asc(tenant_members::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(members.len(), 4);

    let assigned_roles = tenant_member_roles::Entity::find()
        .order_by_asc(tenant_member_roles::Column::Id)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|assignment| (assignment.tenant_member_id, assignment.role_id))
        .collect::<Vec<_>>();
    assert_eq!(assigned_roles, [(1, 1), (2, 3), (3, 4), (4, 5)]);

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
            "billing:manage",
            "documents:read",
            "documents:create",
            "billing:read",
            "billing:manage",
            "analytics:read"
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
            (2, 4),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 1),
            (5, 5),
            (5, 6),
            (5, 7),
            (5, 8),
            (5, 9),
            (6, 5),
            (6, 6),
            (6, 7),
            (6, 8),
            (6, 9),
            (7, 5),
            (7, 6),
            (7, 7),
            (7, 9),
            (8, 5),
            (8, 9)
        ]
    );

    let seeded_documents = documents::Entity::find()
        .order_by_asc(documents::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(
        seeded_documents
            .iter()
            .map(|document| (document.tenant_id, document.title.as_str()))
            .collect::<Vec<_>>(),
        [
            (1, "Designer onboarding"),
            (2, "Developer architecture notes")
        ]
    );

    let seeded_invoices = invoices::Entity::find()
        .order_by_asc(invoices::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(seeded_invoices.len(), 3);
    assert_eq!(seeded_invoices[0].number, "INV-1001");
    assert_eq!(seeded_invoices[1].status, "pending");
    assert_eq!(seeded_invoices[2].number, "INV-DEV-1001");
}
