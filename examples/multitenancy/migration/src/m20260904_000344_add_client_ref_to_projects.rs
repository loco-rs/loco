use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "project_copies",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::Text),
            ],
            &[("tenant", ""), ("client", "")],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "INSERT INTO project_copies \
                 (id, created_at, updated_at, name, description, tenant_id, client_id) \
                 SELECT p.id, p.created_at, p.updated_at, p.name, p.description, p.tenant_id, \
                    (SELECT MIN(c.id) FROM clients c WHERE c.tenant_id = p.tenant_id) \
                 FROM projects p",
            )
            .await?;
        drop_table(m, "projects").await?;
        m.rename_table(
            Table::rename()
                .table(Alias::new("project_copies"), Alias::new("projects"))
                .to_owned(),
        )
        .await?;
        reset_postgres_sequence(m).await?;
        create_project_index(m, true).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "project_copies",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::Text),
            ],
            &[("tenant", "")],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "INSERT INTO project_copies \
                 (id, created_at, updated_at, name, description, tenant_id) \
                 SELECT id, created_at, updated_at, name, description, tenant_id FROM projects",
            )
            .await?;
        drop_table(m, "projects").await?;
        m.rename_table(
            Table::rename()
                .table(Alias::new("project_copies"), Alias::new("projects"))
                .to_owned(),
        )
        .await?;
        reset_postgres_sequence(m).await?;
        create_project_index(m, false).await
    }
}

async fn reset_postgres_sequence(m: &SchemaManager<'_>) -> Result<(), DbErr> {
    if m.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
        m.get_connection()
            .execute_unprepared(
                "SELECT setval(\
                    pg_get_serial_sequence('projects', 'id'), \
                    COALESCE(MAX(id), 1), \
                    MAX(id) IS NOT NULL\
                 ) FROM projects",
            )
            .await?;
    }
    Ok(())
}

async fn create_project_index(m: &SchemaManager<'_>, with_client: bool) -> Result<(), DbErr> {
    let mut index = Index::create();
    index
        .name(if with_client {
            "uidx-projects-tenant-client-name"
        } else {
            "uidx-projects-tenant-name"
        })
        .table(Alias::new("projects"))
        .col(Alias::new("tenant_id"));
    if with_client {
        index.col(Alias::new("client_id"));
    }
    index.col(Alias::new("name")).unique();
    m.create_index(index).await
}
