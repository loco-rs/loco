use loco_rs::prelude::*;

use crate::models::permissions;

/// Requires a tenant member to hold the requested core-resource permission.
///
/// # Errors
///
/// Returns an authorization error when the member lacks the permission, or a
/// database error when the access query fails.
pub async fn require_permission(
    ctx: &AppContext,
    user_id: i64,
    tenant_id: i64,
    permission: &str,
) -> Result<()> {
    if permissions::Model::user_can(&ctx.db, tenant_id, user_id, permission).await? {
        Ok(())
    } else {
        unauthorized("tenant member does not have the required permission")
    }
}
