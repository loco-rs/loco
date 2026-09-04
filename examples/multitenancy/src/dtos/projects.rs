use sea_orm::prelude::DateTimeWithTimeZone;
use ts_rs::TS;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ProjectDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub tenant_id: i64,
    #[ts(type = "number")]
    pub client_id: i64,
    pub client_name: String,
    pub name: String,
    pub description: String,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl ProjectDto {
    #[must_use]
    pub fn from_models(
        model: crate::models::_entities::projects::Model,
        client: crate::models::_entities::clients::Model,
    ) -> Self {
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
            client_id: model.client_id,
            client_name: client.name,
            name: model.name,
            description: model.description,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateProject {
    #[validate(range(min = 1))]
    #[ts(type = "number")]
    pub client_id: i64,
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 1_000))]
    pub description: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateProject {
    #[validate(range(min = 1))]
    #[ts(type = "number")]
    pub client_id: i64,
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 1_000))]
    pub description: String,
}
