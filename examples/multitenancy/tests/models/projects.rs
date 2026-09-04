use loco_rs::{prelude::TenantActiveModelExt, testing::prelude::*};
use multitenancy::{
    app::App,
    models::{
        _entities::{clients, projects},
        tenants, users,
    },
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, ModelTrait};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn project_belongs_to_client() {
    let boot = boot_test::<App>().await.unwrap();
    let owner = users::Model::create_with_password(
        &boot.app_context.db,
        &users::RegisterParams {
            name: "Test Owner".to_owned(),
            email: "owner@example.com".to_owned(),
            password: "password".to_owned(),
        },
    )
    .await
    .unwrap();
    let workspace =
        tenants::Model::create_workspace(&boot.app_context.db, owner.id, "Test workspace")
            .await
            .unwrap();
    let client = clients::ActiveModel {
        name: Set("Acme Studio".to_owned()),
        email: Set("hello@acme.example".to_owned()),
        ..Default::default()
    }
    .set_tenant(workspace.tenant.id)
    .unwrap()
    .insert(&boot.app_context.db)
    .await
    .unwrap();
    let created_project = projects::ActiveModel {
        client_id: Set(client.id),
        name: Set("Brand refresh".to_owned()),
        description: Set("Refresh the visual identity.".to_owned()),
        ..Default::default()
    }
    .set_tenant(workspace.tenant.id)
    .unwrap()
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    let project = projects::Entity::find_by_id(created_project.id)
        .one(&boot.app_context.db)
        .await
        .unwrap()
        .unwrap();
    let client = project
        .find_related(clients::Entity)
        .one(&boot.app_context.db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(client.id, project.client_id);
    assert_eq!(client.name, "Acme Studio");
}
