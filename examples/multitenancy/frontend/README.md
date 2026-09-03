# Multi-tenancy SPA

This Vite + React frontend mirrors the structure of `examples/reference_spa`:
React Router handles navigation, TanStack Query owns server state, and typed
bindings are generated from Rust DTOs with `ts-rs`.

```sh
pnpm install
pnpm dev
```

Vite serves the app at <http://localhost:5173> and proxies `/api` to Loco on
port 5150. For a production-style single-origin build, run `pnpm build` and
then start Loco from the example root. Loco serves `frontend/dist` with SPA
fallback.

The SPA includes registration, login, tenant selection, workspace
creation, and logout. Registration asks only for the user's name, email, and
password, then opens the workspace modal. Tenant slugs are generated from the
workspace name automatically. Login uses Loco's JWT endpoint, and
`/api/auth/workspaces` lists and creates workspaces for the authenticated user.
The authenticated navbar contains an organization-style workspace menu, with
`New workspace` as its final action. It lists each tenant once;
`tenant_applications` is reserved for optional add-on subscriptions.

The workspace console provides Overview, Clients, Projects, Documents, Billing, Members, and
Add-ons pages. Overview summarizes tenant-owned records and access. The
Members page displays each role and its effective tenant permissions.
Each non-owner member links to a dedicated access page, and workspace Owners
can open a separate role-management page from the table. That page assigns a
member's role and configures the selected role's permissions for core resources;
role permission changes affect every member with that role.
Clients, Projects, Documents, and Billing are core areas provisioned for every tenant and are not
part of a subscription. Their
navigation and overview metrics appear only when the current member has the
matching read permission, and their write forms remain hidden for read-only
members. They do not appear in the Add-ons catalog. That page derives optional
product availability from the workspace's purchased subscription. Designer
includes Client Portal and Priority Support, while Developer includes Feature
Flags and Priority Support. Analytics is active only for Developer. All four
add-ons are subscription-only and do not add permissions or role grants.
