#![allow(clippy::missing_errors_doc)]
use std::collections::{BTreeMap, HashMap};

use loco_rs::prelude::*;
use sea_orm::PaginatorTrait;

use crate::{
    dtos::dashboard::{
        DashboardApplication, DashboardDto, DashboardStats, MemberAccess, PermissionAccess,
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
            let Some((application_id, application_name)) =
                access.applications.get(&permission.tenant_application_id)
            else {
                continue;
            };
            effective_permissions.insert(
                (application_name.clone(), permission.key.clone()),
                PermissionAccess {
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

type Subscription = (tenant_applications::Model, Option<applications::Model>);

struct AccessRows {
    member_users: Vec<(tenant_members::Model, Option<users::Model>)>,
    assignments: Vec<tenant_member_roles::Model>,
    roles: Vec<roles::Model>,
    grants: Vec<role_permissions::Model>,
    permissions: Vec<permissions::Model>,
    subscriptions: Vec<Subscription>,
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
            subscriptions: tenant_applications::Entity::find()
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
    applications: HashMap<i64, (i64, String)>,
}

impl AccessMaps {
    fn from_rows(
        assignments: Vec<tenant_member_roles::Model>,
        roles: Vec<roles::Model>,
        grants: Vec<role_permissions::Model>,
        permissions: Vec<permissions::Model>,
        subscriptions: &[Subscription],
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
            applications: subscriptions
                .iter()
                .filter_map(|(subscription, application)| {
                    application.as_ref().map(|application| {
                        (subscription.id, (application.id, application.name.clone()))
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
    subscriptions: Vec<Subscription>,
    current_member: &MemberAccess,
) -> Vec<DashboardApplication> {
    let mut result = subscriptions
        .into_iter()
        .filter_map(|(subscription, application)| {
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
                    status: subscription.status,
                    permissions,
                }
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.name.cmp(&right.name));
    result
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
        subscriptions,
    } = rows;
    let access = AccessMaps::from_rows(assignments, roles, grants, permissions, &subscriptions);
    let members = build_members(member_users, &access);
    let current_member = members
        .iter()
        .find(|member| member.user_id == auth.user.id)
        .cloned()
        .ok_or(ModelError::EntityNotFound)?;

    let dashboard_applications = build_applications(subscriptions, &current_member);

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
        applications: dashboard_applications,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/tenants/{tenant_id}/dashboard/")
        .add("/", get(show))
}
