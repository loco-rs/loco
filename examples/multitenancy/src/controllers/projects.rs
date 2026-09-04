#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;

use crate::{
    controllers::authorization::require_permission,
    dtos::projects::{CreateProject, ProjectDto, UpdateProject},
    models::{clients, projects, users},
};

const VIEW_PROJECTS: &str = "projects:view";
const CREATE_PROJECTS: &str = "projects:create";
const EDIT_PROJECTS: &str = "projects:edit";

async fn tenant_client(ctx: &AppContext, tenant_id: i64, client_id: i64) -> Result<clients::Model> {
    clients::Entity::find_by_id(client_id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound.into())
}

fn project_dto(
    tenant_id: i64,
    project: projects::Model,
    client: Option<clients::Model>,
) -> Result<ProjectDto> {
    let client = client
        .filter(|client| client.tenant_id == tenant_id)
        .ok_or(ModelError::EntityNotFound)?;
    Ok(ProjectDto::from_models(project, client))
}

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_PROJECTS).await?;
    let rows = projects::Entity::find()
        .in_tenant(tenant_id)
        .find_also_related(clients::Entity)
        .all(&ctx.db)
        .await?;
    let response = rows
        .into_iter()
        .map(|(project, client)| project_dto(tenant_id, project, client))
        .collect::<Result<Vec<_>>>()?;
    format::json(response)
}

#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_PROJECTS).await?;
    let (project, client) = projects::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .find_also_related(clients::Entity)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    format::json(project_dto(tenant_id, project, client)?)
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
    JsonValidate(params): JsonValidate<CreateProject>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, CREATE_PROJECTS).await?;
    let client = tenant_client(&ctx, tenant_id, params.client_id).await?;
    let project = projects::ActiveModel {
        client_id: Set(client.id),
        name: Set(params.name),
        description: Set(params.description),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;
    format::json(ProjectDto::from_models(project, client))
}

#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateProject>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, EDIT_PROJECTS).await?;
    let client = tenant_client(&ctx, tenant_id, params.client_id).await?;
    let project = projects::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let mut project = project.into_active_model();
    project.client_id = Set(client.id);
    project.name = Set(params.name);
    project.description = Set(params.description);
    format::json(ProjectDto::from_models(
        project.update(&ctx.db).await?,
        client,
    ))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/projects/")
        .add("/", get(index))
        .add("/", post(create))
        .add("{id}", get(show))
        .add("{id}", put(update))
}
