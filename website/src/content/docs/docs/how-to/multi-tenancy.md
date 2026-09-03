---
title: Add row-level multi-tenancy
description: Scope Sea-ORM reads and writes to a trusted tenant, safely assign tenant keys, and model application subscriptions and RBAC.
sidebar:
  order: 3
---

**Goal:** isolate tenant-owned rows in one shared database while keeping the active tenant explicit in application code.

Loco's tenant helpers follow the row-level approach popularized by `acts_as_tenant`: each owned table has a tenant foreign key and every read or mutation includes that key. Unlike Rails' request-global `current_tenant`, Loco passes the tenant ID explicitly. This is safe across async requests, workers, tasks, and tests, and makes the security boundary visible during review.

For a complete runnable implementation, see [`examples/multitenancy`](https://github.com/loco-rs/loco/tree/master/examples/multitenancy). Its request tests build two tenants and two applications and exercise the isolation and RBAC boundaries below.

## 1. Model the tenancy relationships

Generate the domain tables your application needs. A multi-application SaaS commonly starts with:

```sh
$ cargo loco generate model tenants name:string! slug:string^
$ cargo loco generate model applications name:string^
$ cargo loco generate model tenant_applications tenant:references application:references status:string!
$ cargo loco generate model tenant_members tenant:references user:references
$ cargo loco generate model roles tenant:references name:string!
$ cargo loco generate model tenant_member_roles tenant:references tenant_member:references role:references
$ cargo loco generate model permissions tenant:references tenant_application:references key:string!
$ cargo loco generate model role_permissions tenant:references role:references permission:references
```

This schema gives tenants and applications a many-to-many relationship through `tenant_applications`. `tenant_members` manages membership, `tenant_member_roles` lets members hold multiple roles, and each permission targets a subscribed application rather than a global application. Add the status, invitation, ownership, and audit fields your product requires.

Use migrations to add composite unique constraints and indexes for your access patterns. Typical constraints include `(tenant_id, application_id)` on subscriptions, `(tenant_id, user_id)` on members, `(tenant_id, name)` on roles, `(tenant_id, tenant_application_id, key)` on permissions, and `(tenant_id, role_id, permission_id)` on role permissions. Tenant-scoped uniqueness belongs in the database; an application-only uniqueness check is vulnerable to races.

## 2. Mark tenant-owned entities

Implement `TenantEntity` in each hand-written model module. Do not edit the generated `_entities` file:

```rust
use loco_rs::prelude::*;

pub use super::_entities::documents::{self, ActiveModel, Entity, Model};

impl TenantEntity for documents::Entity {
    type TenantId = i64;

    fn tenant_column() -> documents::Column {
        documents::Column::TenantId
    }
}
```

`TenantId` can match the key type used by your app, such as `i64`, `Uuid`, or `String`. Implement the trait for owned resources and join models (`documents`, `tenant_members`, `roles`, `tenant_applications`, `permissions`, and both role join tables in the example), not for global catalog tables such as `applications`.

## 3. Resolve a trusted tenant

Resolve the tenant from authenticated data, then verify membership before running a scoped query. Do not trust a tenant ID from a path, header, or request body on its own.

```rust
let tenant_id = membership::Entity::find()
    .filter(membership::Column::UserId.eq(auth.user.id))
    .filter(membership::Column::TenantId.eq(requested_tenant_id))
    .one(&ctx.db)
    .await?
    .ok_or(ModelError::EntityNotFound)?
    .tenant_id;
```

The exact resolver is application policy: it might use a URL slug, subdomain, JWT claim, or API-key relationship. Pass the resolved ID into workers as part of their serializable arguments and resolve it again inside the job; there is no ambient request state to leak or clear.

## 4. Scope reads

`in_tenant` composes with ordinary Sea-ORM filters:

```rust
let documents = documents::Entity::find()
    .in_tenant(tenant_id)
    .all(&ctx.db)
    .await?;

let document = documents::Entity::find()
    .filter(documents::Column::Id.eq(document_id))
    .in_tenant(tenant_id)
    .one(&ctx.db)
    .await?
    .ok_or(ModelError::EntityNotFound)?;
```

The generated SQL includes `documents.tenant_id = ?`; a valid ID from another tenant therefore behaves like a missing row.

## 5. Assign the tenant on creation

Use `set_tenant` after building a new active model and before insertion:

```rust
let document = documents::ActiveModel {
    title: Set(params.title),
    ..Default::default()
}
.set_tenant(tenant_id)?
.insert(&ctx.db)
.await?;
```

The helper sets an empty tenant key, accepts an identical key, and returns `ModelError::TenantMismatch` if the model was pre-populated with a different tenant. That prevents request data from overriding the trusted tenant. A tenant mismatch maps to HTTP 400 with a stable `tenant_mismatch` error code.

## 6. Scope updates and deletes

Use scoped bulk builders when changing or deleting an existing tenant-owned row. An active model's normal `.update()` scopes only by its primary key.

```rust
let changes = documents::ActiveModel {
    title: Set(params.title),
    ..Default::default()
};

let updated = documents::Entity::update_many()
    .set(changes)
    .filter(documents::Column::Id.eq(document_id))
    .in_tenant(tenant_id)
    .exec(&ctx.db)
    .await?;

let deleted = documents::Entity::delete_many()
    .filter(documents::Column::Id.eq(document_id))
    .in_tenant(tenant_id)
    .exec(&ctx.db)
    .await?;
```

Check `rows_affected` when your endpoint must distinguish success from a missing or cross-tenant ID.

## 7. Enforce subscriptions and permissions

Tenant isolation and authorization are separate checks. First scope tenant-owned tables with `in_tenant`; then verify that:

1. the user has an active `tenant_members` row,
2. the tenant has an active `tenant_applications` subscription for the requested application, and
3. one of the member's roles has the required permission for that subscription.

Keep the subscription ID in `role_permissions`, not only the global application ID. That ensures a role grants access only to an application currently subscribed by the same tenant. Put the lookup in a model method or Axum middleware so controllers share one policy.

## Intentional unscoped access

Calling Sea-ORM directly without `.in_tenant(...)` remains an explicit escape hatch for cross-tenant administration, reporting, and migrations. Keep that code in clearly named admin services and protect it separately. For defense in depth, production systems can also apply database row-level security where supported.

## Next

- [Query data](/docs/how-to/query-data) for filters that compose with tenant scope.
- [Add a model](/docs/how-to/add-model) for generated entities and migrations.
- [Add middleware](/docs/how-to/add-middleware) if tenant and permission resolution should be shared by a route group.
