#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20260903_183241_tenants;
mod m20260903_183459_applications;
mod m20260903_183504_tenant_applications;
mod m20260903_183509_roles;
mod m20260903_183513_tenant_members;
mod m20260903_183518_tenant_member_roles;
mod m20260903_183522_permissions;
mod m20260903_183527_role_permissions;
mod m20260903_183531_documents;
mod m20260903_184732_add_multitenancy_indexes;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260903_183241_tenants::Migration),
            Box::new(m20260903_183459_applications::Migration),
            Box::new(m20260903_183504_tenant_applications::Migration),
            Box::new(m20260903_183509_roles::Migration),
            Box::new(m20260903_183513_tenant_members::Migration),
            Box::new(m20260903_183518_tenant_member_roles::Migration),
            Box::new(m20260903_183522_permissions::Migration),
            Box::new(m20260903_183527_role_permissions::Migration),
            Box::new(m20260903_183531_documents::Migration),
            Box::new(m20260903_184732_add_multitenancy_indexes::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
