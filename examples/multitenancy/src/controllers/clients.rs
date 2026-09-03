#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;

use crate::{
    controllers::authorization::require_permission,
    dtos::clients::{ClientDto, CreateClient, UpdateClient},
    models::{clients, users},
};

const VIEW_CLIENTS: &str = "clients:view";
const CREATE_CLIENTS: &str = "clients:create";
const EDIT_CLIENTS: &str = "clients:edit";

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_CLIENTS).await?;
    format::json(
        clients::Entity::find()
            .in_tenant(tenant_id)
            .all(&ctx.db)
            .await?
            .into_iter()
            .map(ClientDto::from)
            .collect::<Vec<_>>(),
    )
}

#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_CLIENTS).await?;
    let client = clients::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    format::json(ClientDto::from(client))
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
    JsonValidate(params): JsonValidate<CreateClient>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, CREATE_CLIENTS).await?;
    let client = clients::ActiveModel {
        name: Set(params.name),
        email: Set(params.email),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;
    format::json(ClientDto::from(client))
}

#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateClient>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, EDIT_CLIENTS).await?;
    let client = clients::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let mut client = client.into_active_model();
    client.name = Set(params.name);
    client.email = Set(params.email);
    format::json(ClientDto::from(client.update(&ctx.db).await?))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/clients/")
        .add("/", get(index))
        .add("/", post(create))
        .add("{id}", get(show))
        .add("{id}", put(update))
}
