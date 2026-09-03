#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;
use sea_orm::QueryOrder;

use crate::{
    dtos::invoices::{CreateInvoice, InvoiceDto},
    models::{_entities::tenant_applications, invoices, permissions, users},
};

const READ_BILLING: &str = "billing:read";
const MANAGE_BILLING: &str = "billing:manage";

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

async fn subscription_id(ctx: &AppContext, tenant_id: i64, application_id: i64) -> Result<i64> {
    tenant_applications::Entity::find()
        .in_tenant(tenant_id)
        .filter(tenant_applications::Column::ApplicationId.eq(application_id))
        .filter(tenant_applications::Column::Status.eq("active"))
        .one(&ctx.db)
        .await?
        .map(|subscription| subscription.id)
        .ok_or_else(|| Error::Model(ModelError::EntityNotFound))
}

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, application_id)): Path<(i64, i64)>,
) -> Result<Response> {
    authorize(&ctx, auth.user.id, tenant_id, application_id, READ_BILLING).await?;
    let subscription_id = subscription_id(&ctx, tenant_id, application_id).await?;

    let invoices = invoices::Entity::find()
        .in_tenant(tenant_id)
        .filter(invoices::Column::TenantApplicationId.eq(subscription_id))
        .order_by_desc(invoices::Column::Id)
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(InvoiceDto::from)
        .collect::<Vec<_>>();

    format::json(invoices)
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, application_id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<CreateInvoice>,
) -> Result<Response> {
    authorize(
        &ctx,
        auth.user.id,
        tenant_id,
        application_id,
        MANAGE_BILLING,
    )
    .await?;
    let subscription_id = subscription_id(&ctx, tenant_id, application_id).await?;

    let invoice = invoices::ActiveModel {
        tenant_application_id: Set(subscription_id),
        number: Set(params.number),
        amount_cents: Set(params.amount_cents),
        status: Set(params.status),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&ctx.db)
    .await?;

    format::json(InvoiceDto::from(invoice))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/applications/{application_id}/invoices/")
        .add("/", get(index))
        .add("/", post(create))
}
