use ts_rs::TS;

fn validate_tenant_slug(slug: &str) -> Result<(), validator::ValidationError> {
    if slug.split('-').all(|segment| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    }) {
        Ok(())
    } else {
        let mut error = validator::ValidationError::new("tenant_slug");
        error.message = Some("use lowercase letters, numbers, and single hyphens".into());
        Err(error)
    }
}

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
    #[validate(length(min = 2, max = 100), custom(function = "validate_tenant_slug"))]
    pub tenant_slug: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Workspace {
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub tenant_name: String,
    pub tenant_slug: String,
}
