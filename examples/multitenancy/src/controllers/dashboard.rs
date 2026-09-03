#![allow(clippy::missing_errors_doc)]
use std::collections::{BTreeMap, HashMap};

use loco_rs::prelude::*;
use sea_orm::{JoinType, PaginatorTrait, QuerySelect, RelationTrait};

use crate::{
    dtos::dashboard::{
        DashboardApplication, DashboardDto, DashboardStats, MemberAccess, MemberRoleUpdate,
        PermissionAccess, RoleAccess, RolePermissionsUpdate, UpdateMemberRole,
        UpdateRolePermissions,
    },
    models::_entities::{
        applications, documents, invoices, permissions, role_permissions, roles,
        tenant_applications, tenant_member_roles, tenant_members, tenants, users,
    },
};

fn member_access(
    member: &tenant_members::Model,
    user: users::Model,
    access: &AccessMaps,
) -> MemberAccess {
    let mut role_names = Vec::new();
    let mut effective_permissions = BTreeMap::new();

    for role_id in access.member_roles.get(&member.id).into_iter().flatten() {
        if let Some(role) = access.roles.get(role_id) {
            role_names.push(role.name.clone());
        }
        for permission_id in access.role_permissions.get(role_id).into_iter().flatten() {
            let Some(permission) = access.permissions.get(permission_id) else {
                continue;
            };
            let Some((application_id, application_name, status)) =
                access.applications.get(&permission.tenant_application_id)
            else {
                continue;
            };
            if status != "active" {
                continue;
            }
            effective_permissions.insert(
                (application_name.clone(), permission.key.clone()),
                PermissionAccess {
                    id: permission.id,
                    application_id: *application_id,
                    application_name: application_name.clone(),
                    key: permission.key.clone(),
                },
            );
        }
    }

    role_names.sort();
    MemberAccess {
        member_id: member.id,
        user_id: user.id,
        name: user.name,
        email: user.email,
        roles: role_names,
        permissions: effective_permissions.into_values().collect(),
    }
}

fn permission_access(
    permission: &permissions::Model,
    access: &AccessMaps,
) -> Option<PermissionAccess> {
    let (application_id, application_name, status) =
        access.applications.get(&permission.tenant_application_id)?;
    if status != "active" {
        return None;
    }
    Some(PermissionAccess {
        id: permission.id,
        application_id: *application_id,
        application_name: application_name.clone(),
        key: permission.key.clone(),
    })
}

fn build_available_permissions(access: &AccessMaps) -> Vec<PermissionAccess> {
    let mut result = access
        .permissions
        .values()
        .filter_map(|permission| permission_access(permission, access))
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        (&left.application_name, &left.key).cmp(&(&right.application_name, &right.key))
    });
    result
}

fn build_roles(access: &AccessMaps) -> Vec<RoleAccess> {
    let mut result = access
        .roles
        .values()
        .map(|role| {
            let mut role_permissions = access
                .role_permissions
                .get(&role.id)
                .into_iter()
                .flatten()
                .filter_map(|permission_id| access.permissions.get(permission_id))
                .filter_map(|permission| permission_access(permission, access))
                .collect::<Vec<_>>();
            role_permissions.sort_by(|left, right| {
                (&left.application_name, &left.key).cmp(&(&right.application_name, &right.key))
            });
            RoleAccess {
                id: role.id,
                name: role.name.clone(),
                permissions: role_permissions,
            }
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|role| match role.name.as_str() {
        "Owner" => 0,
        "Administrator" => 1,
        "Manager" => 2,
        "Support" => 3,
        _ => 4,
    });
    result
}

type TenantApplication = (tenant_applications::Model, Option<applications::Model>);

struct AccessRows {
    member_users: Vec<(tenant_members::Model, Option<users::Model>)>,
    assignments: Vec<tenant_member_roles::Model>,
    roles: Vec<roles::Model>,
    grants: Vec<role_permissions::Model>,
    permissions: Vec<permissions::Model>,
    tenant_applications: Vec<TenantApplication>,
}

impl AccessRows {
    async fn load(ctx: &AppContext, tenant_id: i64) -> Result<Self> {
        Ok(Self {
            member_users: tenant_members::Entity::find()
                .in_tenant(tenant_id)
                .find_also_related(users::Entity)
                .all(&ctx.db)
                .await?,
            assignments: tenant_member_roles::Entity::find()
                .in_tenant(tenant_id)
                .all(&ctx.db)
                .await?,
            roles: roles::Entity::find()
                .in_tenant(tenant_id)
                .all(&ctx.db)
                .await?,
            grants: role_permissions::Entity::find()
                .in_tenant(tenant_id)
                .all(&ctx.db)
                .await?,
            permissions: permissions::Entity::find()
                .in_tenant(tenant_id)
                .all(&ctx.db)
                .await?,
            tenant_applications: tenant_applications::Entity::find()
                .in_tenant(tenant_id)
                .find_also_related(applications::Entity)
                .all(&ctx.db)
                .await?,
        })
    }
}

struct AccessMaps {
    member_roles: HashMap<i64, Vec<i64>>,
    roles: HashMap<i64, roles::Model>,
    role_permissions: HashMap<i64, Vec<i64>>,
    permissions: HashMap<i64, permissions::Model>,
    applications: HashMap<i64, (i64, String, String)>,
}

impl AccessMaps {
    fn from_rows(
        assignments: Vec<tenant_member_roles::Model>,
        roles: Vec<roles::Model>,
        grants: Vec<role_permissions::Model>,
        permissions: Vec<permissions::Model>,
        tenant_applications: &[TenantApplication],
    ) -> Self {
        let mut member_roles: HashMap<i64, Vec<i64>> = HashMap::new();
        for assignment in assignments {
            member_roles
                .entry(assignment.tenant_member_id)
                .or_default()
                .push(assignment.role_id);
        }
        let mut role_permissions: HashMap<i64, Vec<i64>> = HashMap::new();
        for grant in grants {
            role_permissions
                .entry(grant.role_id)
                .or_default()
                .push(grant.permission_id);
        }
        Self {
            member_roles,
            roles: roles.into_iter().map(|role| (role.id, role)).collect(),
            role_permissions,
            permissions: permissions
                .into_iter()
                .map(|permission| (permission.id, permission))
                .collect(),
            applications: tenant_applications
                .iter()
                .filter_map(|(tenant_application, application)| {
                    application.as_ref().map(|application| {
                        (
                            tenant_application.id,
                            (
                                application.id,
                                application.name.clone(),
                                tenant_application.status.clone(),
                            ),
                        )
                    })
                })
                .collect(),
        }
    }
}

fn build_members(
    member_users: Vec<(tenant_members::Model, Option<users::Model>)>,
    access: &AccessMaps,
) -> Vec<MemberAccess> {
    let mut members = member_users
        .into_iter()
        .filter_map(|(member, user)| user.map(|user| member_access(&member, user, access)))
        .collect::<Vec<_>>();
    members.sort_by(|left, right| left.name.cmp(&right.name));
    members
}

fn build_applications(
    tenant_applications: Vec<TenantApplication>,
    current_member: &MemberAccess,
) -> Vec<DashboardApplication> {
    let mut result = tenant_applications
        .into_iter()
        .filter_map(|(tenant_application, application)| {
            application.map(|application| {
                let mut permissions = current_member
                    .permissions
                    .iter()
                    .filter(|permission| permission.application_id == application.id)
                    .map(|permission| permission.key.clone())
                    .collect::<Vec<_>>();
                permissions.sort();
                DashboardApplication {
                    id: application.id,
                    name: application.name,
                    status: tenant_application.status,
                    permissions,
                }
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.name.cmp(&right.name));
    result
}

async fn ensure_owner(ctx: &AppContext, tenant_id: i64, user_id: i64) -> Result<()> {
    let rows = AccessRows::load(ctx, tenant_id).await?;
    let AccessRows {
        member_users,
        assignments,
        roles,
        grants,
        permissions,
        tenant_applications,
    } = rows;
    let access = AccessMaps::from_rows(
        assignments,
        roles,
        grants,
        permissions,
        &tenant_applications,
    );
    let is_owner = build_members(member_users, &access)
        .iter()
        .find(|member| member.user_id == user_id)
        .is_some_and(|member| member.roles.iter().any(|role| role == "Owner"));
    if is_owner {
        Ok(())
    } else {
        unauthorized("only workspace owners can manage roles and permissions")
    }
}

#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path(tenant_id): Path<i64>,
) -> Result<Response> {
    let tenant = tenants::Entity::find_by_id(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    let rows = AccessRows::load(&ctx, tenant_id).await?;
    if !rows
        .member_users
        .iter()
        .any(|(member, _)| member.user_id == auth.user.id)
    {
        return unauthorized("user is not a member of this tenant");
    }

    let AccessRows {
        member_users,
        assignments,
        roles,
        grants,
        permissions,
        tenant_applications,
    } = rows;
    let access = AccessMaps::from_rows(
        assignments,
        roles,
        grants,
        permissions,
        &tenant_applications,
    );
    let members = build_members(member_users, &access);
    let current_member = members
        .iter()
        .find(|member| member.user_id == auth.user.id)
        .cloned()
        .ok_or(ModelError::EntityNotFound)?;

    let dashboard_applications = build_applications(tenant_applications, &current_member);
    let dashboard_roles = build_roles(&access);
    let available_permissions = build_available_permissions(&access);

    let document_count = documents::Entity::find()
        .in_tenant(tenant_id)
        .count(&ctx.db)
        .await?;
    let invoice_count = invoices::Entity::find()
        .in_tenant(tenant_id)
        .count(&ctx.db)
        .await?;
    let stats = DashboardStats {
        member_count: members.len() as u64,
        application_count: dashboard_applications.len() as u64,
        document_count,
        invoice_count,
    };

    format::json(DashboardDto {
        tenant_id,
        tenant_name: tenant.name,
        stats,
        current_member,
        members,
        roles: dashboard_roles,
        available_permissions,
        applications: dashboard_applications,
    })
}

#[debug_handler]
pub async fn update_role_permissions(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, role_id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateRolePermissions>,
) -> Result<Response> {
    ensure_owner(&ctx, tenant_id, auth.user.id).await?;

    roles::Entity::find_by_id(role_id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;

    let permission_ids = params
        .permission_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let tenant_permissions = if permission_ids.is_empty() {
        Vec::new()
    } else {
        permissions::Entity::find()
            .in_tenant(tenant_id)
            .filter(permissions::Column::Id.is_in(permission_ids.clone()))
            .join(
                JoinType::InnerJoin,
                permissions::Relation::TenantApplications.def(),
            )
            .filter(tenant_applications::Column::TenantId.eq(tenant_id))
            .filter(tenant_applications::Column::Status.eq("active"))
            .all(&ctx.db)
            .await?
    };
    if tenant_permissions.len() != permission_ids.len() {
        return bad_request("permissions must belong to the selected workspace");
    }

    let txn = ctx.db.begin().await?;
    role_permissions::Entity::delete_many()
        .filter(role_permissions::Column::TenantId.eq(tenant_id))
        .filter(role_permissions::Column::RoleId.eq(role_id))
        .exec(&txn)
        .await?;
    for permission_id in &permission_ids {
        role_permissions::ActiveModel {
            role_id: Set(role_id),
            permission_id: Set(*permission_id),
            ..Default::default()
        }
        .set_tenant(tenant_id)?
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;

    format::json(RolePermissionsUpdate {
        role_id,
        permission_ids,
    })
}

#[debug_handler]
pub async fn update_member_role(
    State(ctx): State<AppContext>,
    auth: auth::JWTWithUser<users::Model>,
    Path((tenant_id, member_id)): Path<(i64, i64)>,
    JsonValidate(params): JsonValidate<UpdateMemberRole>,
) -> Result<Response> {
    ensure_owner(&ctx, tenant_id, auth.user.id).await?;

    let member = tenant_members::Entity::find_by_id(member_id)
        .in_tenant(tenant_id)
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    if member.user_id == auth.user.id {
        return bad_request("owners cannot change their own role");
    }
    let role = roles::Entity::find()
        .in_tenant(tenant_id)
        .filter(roles::Column::Name.eq(&params.role))
        .one(&ctx.db)
        .await?
        .ok_or(ModelError::EntityNotFound)?;

    let txn = ctx.db.begin().await?;
    tenant_member_roles::Entity::delete_many()
        .filter(tenant_member_roles::Column::TenantId.eq(tenant_id))
        .filter(tenant_member_roles::Column::TenantMemberId.eq(member.id))
        .exec(&txn)
        .await?;
    tenant_member_roles::ActiveModel {
        tenant_member_id: Set(member.id),
        role_id: Set(role.id),
        ..Default::default()
    }
    .set_tenant(tenant_id)?
    .insert(&txn)
    .await?;
    txn.commit().await?;

    format::json(MemberRoleUpdate {
        member_id,
        role: params.role,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/dashboard/")
        .add("/", get(show))
        .add("/members/{member_id}/role", post(update_member_role))
        .add(
            "/roles/{role_id}/permissions",
            post(update_role_permissions),
        )
}
