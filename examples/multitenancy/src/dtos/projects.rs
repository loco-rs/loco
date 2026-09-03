use sea_orm::prelude::DateTimeWithTimeZone;
use ts_rs::TS;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ProjectDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub name: String,
    pub description: String,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<crate::models::_entities::projects::Model> for ProjectDto {
    fn from(model: crate::models::_entities::projects::Model) -> Self {
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
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
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 1_000))]
    pub description: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateProject {
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 1_000))]
    pub description: String,
}
