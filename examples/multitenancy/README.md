# Loco multi-tenancy example

This full-stack application demonstrates shared-database, row-level
multi-tenancy with Loco and Sea-ORM. Its frontend follows the
`reference_spa` stack: Vite, React 19, React Router, TanStack Query, and
TypeScript bindings generated from Rust DTOs with `ts-rs`. It covers the
complete domain from [issue #1640](https://github.com/loco-rs/loco/issues/1640):

- tenants subscribe to optional add-ons through `tenant_applications`;
- users join tenants through memberships;
- memberships have many tenant-owned roles;
- roles receive tenant-level permissions for core resources;
- tenant-owned clients, projects, documents, and invoices are isolated on reads and writes.

The schema lives in [`migration/src`](migration/src). Generated Sea-ORM
entities are under [`src/models/_entities`](src/models/_entities), while the
hand-written modules in [`src/models`](src/models) implement `TenantEntity`.
`permissions::Model::user_can` is the RBAC query, and
[`src/controllers/documents.rs`](src/controllers/documents.rs) and
[`src/controllers/invoices.rs`](src/controllers/invoices.rs) show APIs
that check a login JWT, tenant membership, role, and
permission before accessing tenant-scoped rows. The dashboard endpoint assembles the
workspace's members, roles, effective permissions, add-on availability,
and record counts. [`frontend`](frontend) consumes those APIs as a typed SPA
with registration, login, workspace selection, and logout.

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
Open <http://localhost:5150> and log in with `john@example.com` / `password`.
John Doe is the seeded Owner of both Designer and Developer. The Designer
dashboard includes Jane Smith as Manager and Sam Lee as Support so their
effective permissions can be compared. Both workspaces have the core Clients,
Projects, Documents, and Billing areas. Their optional add-on subscriptions differ: Designer has
Client Portal and Priority Support, Developer has Feature Flags and Priority
Support, and Analytics is active only for Developer. Client Portal, Feature
Flags, and Priority Support demonstrate add-on subscription availability without
creating permissions or role grants. Designer has two clients, two projects,
one document, and two invoices; Developer has one of each core resource. You can also register an
account with your name, email, and password. After registration, the
workspace modal opens so you can name your first tenant; its slug is generated
automatically. Workspace creation atomically adds the tenant, Owner,
Administrator, Manager, and Support roles, assigns the creator as Owner, and
provisions the core Clients, Projects, Documents, and Billing permissions with role-appropriate
permissions. These core features are always available and are not part of a
subscription.

The `--reset` flag makes repeated demo setup predictable by clearing existing
rows before loading the fixed-ID fixtures. It deletes accounts and tenants you
previously created in this example. If the database is already seeded and you
want to keep its data, skip the seed command and run `cargo loco start`.

For frontend development, run Loco on port 5150 and `pnpm dev` from
`frontend`; Vite serves <http://localhost:5173> and proxies `/api` to Loco.
The navbar workspace menu lists Designer and Developer once each. Core feature
access is permission-based, while add-on availability is derived from active
`tenant_applications` rows.

The API endpoints are:

```text
POST /api/auth/register-account
POST /api/auth/login
GET  /api/auth/workspaces
POST /api/auth/workspaces
GET  /api/tenants/{tenant_id}/dashboard
POST /api/tenants/{tenant_id}/dashboard/members/{member_id}/role
POST /api/tenants/{tenant_id}/dashboard/roles/{role_id}/permissions
GET|POST /api/tenants/{tenant_id}/clients
GET|PUT  /api/tenants/{tenant_id}/clients/{id}
GET|POST /api/tenants/{tenant_id}/projects
GET|PUT  /api/tenants/{tenant_id}/projects/{id}
GET|POST /api/tenants/{tenant_id}/documents
GET|PUT  /api/tenants/{tenant_id}/documents/{id}
GET|POST /api/tenants/{tenant_id}/invoices
```

The workspace and document endpoints expect `Authorization: Bearer <jwt>`.
Creating a workspace accepts `{"tenant_name":"Research team","tenant_slug":"research-team"}`;
the SPA derives the slug from the name automatically and selects the new
workspace after creation.
The document POST body is `{"title":"Launch plan"}`. The invoice POST body is
`{"number":"INV-1003","amount_cents":7900,"status":"draft"}`. The SPA stores the JWT and selected
tenant context in local storage; it never includes `tenant_id` in
a create body. Its authenticated console has Overview, Clients, Projects, Documents,
Billing, Members, and Add-ons pages. Core-resource navigation and metrics are
permission-aware, while the Add-ons catalog reflects optional product
availability from the workspace subscription. The seeded catalog includes
Analytics, Client Portal, Feature Flags, and Priority Support; subscription-only
add-ons do not require permissions. The Members table can display
each member's complete effective access on a dedicated page, while workspace
Owners can use a separate management page to assign Owner, Administrator,
Manager, or Support to other members and configure each role's permissions.
Permission changes apply to every workspace member with that role. The request
tests exercise both the seeded role matrix and a separate two-tenant scenario:

```sh
cargo test --test mod
cd frontend && pnpm test
```

They prove that one add-on can be enabled for multiple tenants, that core resource
operations honor permissions, that membership does not transfer to another tenant,
and that client, project, document, and invoice rows receive
the trusted tenant context. They also verify Owner, Manager, and Support Billing
boundaries and prevent non-members from reading the dashboard.
Role-management tests also verify Owner authorization, tenant isolation,
deduplication, and request-size validation for permission grants.

The frontend test command enforces 100% statement, branch, function, and line
coverage for session storage, permission-aware navigation, workspace
navigation, slug generation, and the authenticated route/API-client
boundaries.

## Security model

`TenantQueryExt::in_tenant` deliberately keeps scope at the query call site;
there is no process-global or thread-local current tenant to leak between async
requests. `TenantActiveModelExt::set_tenant` assigns new rows and rejects a
pre-populated, conflicting tenant. Direct unscoped Sea-ORM access remains
available for intentional administration and migrations and should be kept out
of tenant-facing request paths.
