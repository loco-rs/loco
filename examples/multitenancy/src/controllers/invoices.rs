#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;
use sea_orm::QueryOrder;

use crate::{
    controllers::authorization::require_permission,
    dtos::invoices::InvoiceDto,
    models::{invoices, users},
};

const VIEW_BILLING: &str = "billing:view";

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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/invoices/")
        .add("/", get(index))
}
