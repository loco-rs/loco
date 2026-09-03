#![allow(clippy::missing_errors_doc)]
use loco_rs::prelude::*;
use sea_orm::sea_query::Expr;

use crate::{
    controllers::authorization::require_permission,
    dtos::invoices::InvoiceDto,
    models::{applications, invoices, tenant_applications, users},
};

const PURCHASE_ADDONS: &str = "billing:purchase";

fn addon_price(name: &str) -> i64 {
    match name {
        "Analytics" => 4_900,
        "Approval Workflows" => 2_900,
        "Feature Flags" => 3_900,
        "Priority Support" => 9_900,
        _ => 1_900,
    }
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, application_id)): Path<(i64, i64)>,
) -> Result<Response> {
    require_permission(&ctx, auth.user.id, tenant_id, PURCHASE_ADDONS).await?;

    let txn = ctx.db.begin().await?;
    let (subscription, application) = tenant_applications::Entity::find()
        .in_tenant(tenant_id)
        .filter(tenant_applications::Column::ApplicationId.eq(application_id))
        .find_also_related(applications::Entity)
        .one(&txn)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let application = application.ok_or(ModelError::EntityNotFound)?;
    if subscription.status == "active" {
        return bad_request("add-on is already included in this workspace");
    }

    let update = tenant_applications::Entity::update_many()
        .col_expr(tenant_applications::Column::Status, Expr::value("active"))
        .filter(tenant_applications::Column::Id.eq(subscription.id))
        .filter(tenant_applications::Column::Status.ne("active"))
        .exec(&txn)
        .await?;
    if update.rows_affected != 1 {
        return bad_request("add-on is already included in this workspace");
    }

    let invoice = invoices::ActiveModel {
        number: Set(format!(
            "ADDON-{tenant_id}-{application_id}-{}",
            chrono::Utc::now().timestamp_millis()
        )),
        description: Set(format!("{} add-on purchase", application.name)),
        amount_cents: Set(addon_price(&application.name)),
        status: Set("paid".to_owned()),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&txn)
    .await?;
    txn.commit().await?;

    format::json(InvoiceDto::from(invoice))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/addons/")
        .add("{application_id}/purchase", post(create))
}
