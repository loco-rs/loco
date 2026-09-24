---
title: Add row-level multi-tenancy
description: Enable tenant scoping for Sea-ORM queries and safely assign tenant keys when creating records.
sidebar:
  order: 3
---

**Goal:** isolate tenant-owned rows in one shared database while keeping the active tenant explicit in application code.

Loco provides three opt-in traits: `TenantEntity` identifies an entity's tenant column, `TenantQueryExt::in_tenant` filters queries, and `TenantActiveModelExt::set_tenant` assigns the tenant on an active model. The tenant ID is passed explicitly, so these helpers do not depend on request-global or thread-local state.

## 1. Enable the feature

Add `multi-tenancy` to the features of your existing `loco-rs` dependency:

```toml
loco-rs = { version = "1", features = ["multi-tenancy"] }
```

This feature is disabled by default and enables `with-db`. It makes the traits available from both `loco_rs::model` and `loco_rs::prelude`.

## 2. Mark tenant-owned entities

Generate a tenant table and a resource with a tenant foreign key:

```sh
$ cargo loco generate model tenants name:string!
$ cargo loco generate model documents title:string! tenant:references
```

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

`TenantId` must match the type stored in the tenant column, such as `i64`, `Uuid`, or `String`. Only implement the trait on tenant-owned entities. Use migrations to add indexes and composite unique constraints for tenant-specific values, such as `(tenant_id, title)` when document titles must be unique within a tenant.

## 3. Resolve a trusted tenant

Resolve the tenant from authenticated data, then verify membership before running a scoped query. Do not trust a tenant ID from a path, header, or request body on its own.

Tenant resolution and permissions remain application policy. A resolver might use a URL slug, subdomain, JWT claim, or API-key relationship, but must authorize the caller for that tenant. In the examples below, `tenant_id` is the result of this check. Workers can carry the tenant ID in their serializable arguments and validate access when the job runs.

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

## Scope and limitations

These traits do not automatically scope every database operation. Calling Sea-ORM without `.in_tenant(...)` remains unscoped, which is useful for authorized cross-tenant administration, reporting, and migrations. Audit tenant-owned queries for the filter.

`in_tenant` filters the target entity; it does not scope joined tables or validate referenced rows. Validate that related records belong to the same tenant and enforce this in the database where possible. Build mutation fields explicitly so clients cannot change tenant ownership through bulk updates. The helpers do not install database row-level security or provide membership, role, or subscription management.

## Next

- [Query data](/docs/how-to/query-data) for filters that compose with tenant scope.
- [Add a model](/docs/how-to/add-model) for generated entities and migrations.
- [Add middleware](/docs/how-to/add-middleware) if tenant and permission resolution should be shared by a route group.
