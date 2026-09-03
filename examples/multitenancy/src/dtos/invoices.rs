use sea_orm::prelude::DateTimeWithTimeZone;
use ts_rs::TS;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct InvoiceDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub number: String,
    pub description: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub status: String,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<crate::models::_entities::invoices::Model> for InvoiceDto {
    fn from(model: crate::models::_entities::invoices::Model) -> Self {
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
            number: model.number,
            description: model.description,
            amount_cents: model.amount_cents,
            status: model.status,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
