use loco_rs::testing::prelude::*;
use multitenancy::{
    app::App,
    models::_entities::{clients, projects},
};
use sea_orm::{EntityTrait, ModelTrait};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn project_belongs_to_client() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let project = projects::Entity::find_by_id(1)
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
