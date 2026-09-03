# Loco multi-tenancy example

This application demonstrates shared-database, row-level multi-tenancy with
Loco and Sea-ORM. It covers the complete domain from
[issue #1640](https://github.com/loco-rs/loco/issues/1640):

- tenants subscribe to many applications and applications serve many tenants;
- users join tenants through memberships;
- memberships have many tenant-owned roles;
- roles receive permissions tied to a tenant's active application subscription;
- tenant-owned documents are isolated on reads and writes.

The schema lives in [`migration/src`](migration/src). Generated Sea-ORM
entities are under [`src/models/_entities`](src/models/_entities), while the
hand-written modules in [`src/models`](src/models) implement `TenantEntity`.
`permissions::Model::user_can` is the RBAC query, and
[`src/controllers/documents.rs`](src/controllers/documents.rs) shows an API
that checks an API token, tenant membership, subscription, role, and permission
before accessing tenant-scoped rows.

## Run it

From this directory:

```sh
cargo loco db migrate
cargo loco start
```

The example inherits the framework from the repository root through a local
path dependency, so it always exercises the code in the current checkout.

The API endpoints are:

```text
GET  /api/tenants/{tenant_id}/applications/{application_id}/documents
POST /api/tenants/{tenant_id}/applications/{application_id}/documents
```

Both expect `Authorization: Bearer <user-api-key>`. The POST body is
`{"title":"Launch plan"}`. The request tests create a complete two-tenant,
two-application scenario and are the quickest executable walkthrough:

```sh
cargo test --test mod requests::documents
```

They prove that one application can be subscribed by multiple tenants, that a
role's permission does not transfer to another application, that membership
does not transfer to another tenant, and that created rows receive the trusted
tenant ID.

## Security model

`TenantQueryExt::in_tenant` deliberately keeps scope at the query call site;
there is no process-global or thread-local current tenant to leak between async
requests. `TenantActiveModelExt::set_tenant` assigns new rows and rejects a
pre-populated, conflicting tenant. Direct unscoped Sea-ORM access remains
available for intentional administration and migrations and should be kept out
of tenant-facing request paths.
