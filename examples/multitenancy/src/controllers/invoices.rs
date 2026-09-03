#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;
use sea_orm::QueryOrder;

use crate::{
    controllers::authorization::require_permission,
    dtos::invoices::{CreateInvoice, InvoiceDto},
    models::{invoices, users},
};

const VIEW_BILLING: &str = "billing:view";
const CREATE_BILLING: &str = "billing:create";

#[debug_handler]
pub async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, VIEW_BILLING).await?;
    format::json(
        invoices::Entity::find()
            .in_tenant(tenant_id)
            .order_by_desc(invoices::Column::Id)
            .all(&ctx.db)
            .await?
            .into_iter()
            .map(InvoiceDto::from)
            .collect::<Vec<_>>(),
    )
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
    JsonValidate(params): JsonValidate<CreateInvoice>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, CREATE_BILLING).await?;
    let invoice = invoices::ActiveModel {
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
        .prefix("api/tenants/{tenant_id}/invoices/")
        .add("/", get(index))
        .add("/", post(create))
}
