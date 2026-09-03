use ts_rs::TS;

fn validate_role(role: &str) -> Result<(), validator::ValidationError> {
    if matches!(role, "Owner" | "Manager" | "Viewer") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("role"))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PermissionAccess {
    #[ts(type = "number")]
    pub application_id: i64,
    pub application_name: String,
    pub key: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct MemberAccess {
    #[ts(type = "number")]
    pub member_id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<PermissionAccess>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DashboardApplication {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub status: String,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DashboardStats {
    #[ts(type = "number")]
    pub member_count: u64,
    #[ts(type = "number")]
    pub application_count: u64,
    #[ts(type = "number")]
    pub document_count: u64,
    #[ts(type = "number")]
    pub invoice_count: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DashboardDto {
    #[ts(type = "number")]
    pub tenant_id: i64,
    pub tenant_name: String,
    pub stats: DashboardStats,
    pub current_member: MemberAccess,
    pub members: Vec<MemberAccess>,
    pub applications: Vec<DashboardApplication>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateMemberRole {
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct MemberRoleUpdate {
    #[ts(type = "number")]
    pub member_id: i64,
    pub role: String,
}
