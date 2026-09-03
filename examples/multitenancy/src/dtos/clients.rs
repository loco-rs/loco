use sea_orm::prelude::DateTimeWithTimeZone;
use ts_rs::TS;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ClientDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub name: String,
    pub email: String,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<crate::models::_entities::clients::Model> for ClientDto {
    fn from(model: crate::models::_entities::clients::Model) -> Self {
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
            name: model.name,
            email: model.email,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateClient {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateClient {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}
