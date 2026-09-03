#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;

use crate::{
    controllers::authorization::require_permission,
    dtos::projects::{CreateProject, ProjectDto, UpdateProject},
    models::{projects, users},
};

const VIEW_PROJECTS: &str = "projects:view";
const CREATE_PROJECTS: &str = "projects:create";
const EDIT_PROJECTS: &str = "projects:edit";

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_PROJECTS).await?;
    format::json(
        projects::Entity::find()
            .in_tenant(tenant_id)
            .all(&ctx.db)
            .await?
            .into_iter()
            .map(ProjectDto::from)
            .collect::<Vec<_>>(),
    )
}

#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_PROJECTS).await?;
    let project = projects::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    format::json(ProjectDto::from(project))
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
    JsonValidate(params): JsonValidate<CreateProject>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, CREATE_PROJECTS).await?;
    let project = projects::ActiveModel {
        name: Set(params.name),
        description: Set(params.description),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;
    format::json(ProjectDto::from(project))
}

#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateProject>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, EDIT_PROJECTS).await?;
    let project = projects::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let mut project = project.into_active_model();
    project.name = Set(params.name);
    project.description = Set(params.description);
    format::json(ProjectDto::from(project.update(&ctx.db).await?))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/projects/")
        .add("/", get(index))
        .add("/", post(create))
        .add("{id}", get(show))
        .add("{id}", put(update))
}
