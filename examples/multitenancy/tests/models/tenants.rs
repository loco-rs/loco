use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::_entities::{
        applications, clients, documents, invoices, permissions, projects, role_permissions, roles,
        tenant_applications, tenant_member_roles, tenant_members, tenants, users,
    },
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
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
            (1, 1, "inactive"),
            (1, 2, "active"),
            (1, 3, "inactive"),
            (1, 4, "active"),
            (2, 1, "active"),
            (2, 2, "inactive"),
            (2, 3, "active"),
            (2, 4, "active")
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
        .filter(permissions::Column::TenantId.eq(1))
        .order_by_asc(permissions::Column::Key)
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|permission| permission.key)
        .collect();
    assert_eq!(
        permission_keys,
        [
            "billing:create",
            "billing:view",
            "clients:create",
            "clients:edit",
            "clients:view",
            "documents:create",
            "documents:edit",
            "documents:view",
            "projects:create",
            "projects:edit",
            "projects:view"
        ]
    );

    assert_eq!(permissions::Entity::find().count(db).await.unwrap(), 22);
    for (role_id, expected_grants) in [
        (1, 11),
        (2, 11),
        (3, 10),
        (4, 3),
        (5, 11),
        (6, 11),
        (7, 10),
        (8, 3),
    ] {
        assert_eq!(
            role_permissions::Entity::find()
                .filter(role_permissions::Column::RoleId.eq(role_id))
                .count(db)
                .await
                .unwrap(),
            expected_grants
        );
    }

    assert_eq!(clients::Entity::find().count(db).await.unwrap(), 3);
    assert_eq!(projects::Entity::find().count(db).await.unwrap(), 3);

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
