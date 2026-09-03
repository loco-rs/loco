#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::{documents, permissions, users};

const READ_DOCUMENTS: &str = "documents:read";
const CREATE_DOCUMENTS: &str = "documents:create";

#[derive(Debug, Deserialize)]
pub struct CreateParams {
    title: String,
}

async fn authorize(
    ctx: &AppContext,
    user_id: i64,
    tenant_id: i64,
    application_id: i64,
    permission: &str,
) -> Result<()> {
    if permissions::Model::user_can(&ctx.db, tenant_id, user_id, application_id, permission).await?
    {
        Ok(())
    } else {
        unauthorized("tenant member does not have the required application permission")
    }
}

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::ApiToken<users::Model>,
    Path((tenant_id, application_id)): Path<(i64, i64)>,
) -> Result<Response> {
    authorize(
        &ctx,
        auth.user.id,
        tenant_id,
        application_id,
        READ_DOCUMENTS,
    )
    .await?;

    format::json(
        documents::Entity::find()
            .in_tenant(tenant_id)
            .all(&ctx.db)
            .await?,
    )
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::ApiToken<users::Model>,
    Path((tenant_id, application_id)): Path<(i64, i64)>,
    Json(params): Json<CreateParams>,
) -> Result<Response> {
    authorize(
        &ctx,
        auth.user.id,
        tenant_id,
        application_id,
        CREATE_DOCUMENTS,
    )
    .await?;

    let document = documents::ActiveModel {
        title: Set(params.title),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;

    format::json(document)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/applications/{application_id}/documents/")
        .add("/", get(index))
        .add("/", post(create))
}
