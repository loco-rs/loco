#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;

use crate::{
    controllers::authorization::require_permission,
    dtos::documents::{CreateDocument, DocumentDto, UpdateDocument},
    models::{documents, users},
};

const VIEW_DOCUMENTS: &str = "documents:view";
const CREATE_DOCUMENTS: &str = "documents:create";
const EDIT_DOCUMENTS: &str = "documents:edit";

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_DOCUMENTS).await?;
    format::json(
        documents::Entity::find()
            .in_tenant(tenant_id)
            .all(&ctx.db)
            .await?
            .into_iter()
            .map(DocumentDto::from)
            .collect::<Vec<_>>(),
    )
}

#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_DOCUMENTS).await?;
    let document = documents::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    format::json(DocumentDto::from(document))
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
    JsonValidate(params): JsonValidate<CreateDocument>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, CREATE_DOCUMENTS).await?;
    let document = documents::ActiveModel {
        title: Set(params.title),
        description: Set(params.description),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;
    format::json(DocumentDto::from(document))
}

#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateDocument>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, EDIT_DOCUMENTS).await?;
    let document = documents::Entity::find_by_id(id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let mut document = document.into_active_model();
    document.title = Set(params.title);
    document.description = Set(params.description);
    format::json(DocumentDto::from(document.update(&ctx.db).await?))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/documents/")
        .add("/", get(index))
        .add("/", post(create))
        .add("{id}", get(show))
        .add("{id}", put(update))
}
