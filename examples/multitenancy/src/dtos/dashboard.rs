use ts_rs::TS;

fn validate_role(role: &str) -> Result<(), validator::ValidationError> {
    if matches!(role, "Owner" | "Administrator" | "Manager" | "Support") {
        Ok(())
    } else {
        Err(validator::ValidationError::new("role"))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PermissionAccess {
    #[ts(type = "number")]
    pub id: i64,
    pub key: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RoleAccess {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub permissions: Vec<PermissionAccess>,
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
pub struct DashboardAddon {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub status: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct DashboardStats {
    #[ts(type = "number")]
    pub member_count: u64,
    #[ts(type = "number")]
    pub addon_count: u64,
    #[ts(type = "number")]
    pub client_count: u64,
    #[ts(type = "number")]
    pub project_count: u64,
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
    pub roles: Vec<RoleAccess>,
    pub available_permissions: Vec<PermissionAccess>,
    pub addons: Vec<DashboardAddon>,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, validator::Validate, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateRolePermissions {
    #[validate(length(max = 100))]
    #[ts(type = "Array<number>")]
    pub permission_ids: Vec<i64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RolePermissionsUpdate {
    #[ts(type = "number")]
    pub role_id: i64,
    #[ts(type = "Array<number>")]
    pub permission_ids: Vec<i64>,
}
