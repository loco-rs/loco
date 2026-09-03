use ts_rs::TS;

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RegisterAccount {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateWorkspace {
    #[validate(length(min = 2, max = 100))]
    pub tenant_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Workspace {
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub tenant_name: String,
    pub tenant_slug: String,
}
