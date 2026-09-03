# Loco multi-tenancy example

This full-stack application demonstrates shared-database, row-level
multi-tenancy with Loco and Sea-ORM. Its frontend follows the
`reference_spa` stack: Vite, React 19, React Router, TanStack Query, and
TypeScript bindings generated from Rust DTOs with `ts-rs`. It covers the
complete domain from [issue #1640](https://github.com/loco-rs/loco/issues/1640):

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
that checks a login JWT, tenant membership, subscription, role, and permission
before accessing tenant-scoped rows. [`frontend`](frontend) consumes that API
as a typed SPA with registration, login, workspace selection, and logout.

## Run it

For a single-origin production-style run, from this directory:

```sh
cargo loco db migrate
cargo loco db seed --reset
cd frontend
pnpm install
pnpm build
cd ..
cargo loco start
```

The example inherits the framework from the repository root through a local
path dependency, so it always exercises the code in the current checkout.
Open <http://localhost:5150> and log in with `user1@example.com` / `12341234`.
The seeded editor can list and create Acme documents. You can also register an
account with your name, email, and password. After registration, the workspace
modal opens so you can name your first tenant; its slug is generated
automatically. Workspace creation atomically adds the tenant, owner membership,
Documents subscription, and read/create permissions.

The `--reset` flag makes repeated demo setup predictable by clearing existing
rows before loading the fixed-ID fixtures. It deletes accounts and tenants you
previously created in this example. If the database is already seeded and you
want to keep its data, skip the seed command and run `cargo loco start`.

For frontend development, run Loco on port 5150 and `pnpm dev` from
`frontend`; Vite serves <http://localhost:5173> and proxies `/api` to Loco.

The API endpoints are:

```text
POST /api/auth/register-account
POST /api/auth/login
GET  /api/auth/workspaces
POST /api/auth/workspaces
GET  /api/tenants/{tenant_id}/applications/{application_id}/documents
POST /api/tenants/{tenant_id}/applications/{application_id}/documents
```

The workspace and document endpoints expect `Authorization: Bearer <jwt>`.
Creating a workspace accepts `{"tenant_name":"Research team","tenant_slug":"research-team"}`;
the SPA derives the slug from the name automatically and selects the new
workspace after creation.
The document POST body is
`{"title":"Launch plan"}`. The SPA stores the JWT and selected
tenant/application context in local storage; it never includes `tenant_id` in
a create body. The request tests create a complete two-tenant,
two-application scenario and are the quickest executable walkthrough:

```sh
cargo test --test mod requests::documents
cd frontend && pnpm test
```

They prove that one application can be subscribed by multiple tenants, that a
role's permission does not transfer to another application, that membership
does not transfer to another tenant, and that created rows receive the trusted
tenant ID.

The frontend test command enforces 100% statement, branch, function, and line
coverage for session/workspace storage and the authenticated API-client
boundary.

## Security model

`TenantQueryExt::in_tenant` deliberately keeps scope at the query call site;
there is no process-global or thread-local current tenant to leak between async
requests. `TenantActiveModelExt::set_tenant` assigns new rows and rejects a
pre-populated, conflicting tenant. Direct unscoped Sea-ORM access remains
available for intentional administration and migrations and should be kept out
of tenant-facing request paths.
