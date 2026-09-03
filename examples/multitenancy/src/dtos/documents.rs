use sea_orm::prelude::DateTimeWithTimeZone;
use ts_rs::TS;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DocumentDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub title: String,
    pub description: String,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<crate::models::_entities::documents::Model> for DocumentDto {
    fn from(model: crate::models::_entities::documents::Model) -> Self {
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
            title: model.title,
            description: model.description,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateDocument {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 2, max = 2_000))]
    pub description: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateDocument {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 2, max = 2_000))]
    pub description: String,
}
