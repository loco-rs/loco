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
password, then opens the workspace modal. The server generates tenant slugs
from the workspace name and scopes them by tenant ID, so duplicate names are
safe and users never type slugs directly. Login and registration are public-only
routes; an authenticated user is redirected to the dashboard. Login uses Loco's JWT endpoint, and
`/api/auth/workspaces` lists and creates workspaces for the authenticated user.
The authenticated navbar contains an organization-style workspace menu, with
`New workspace` as its final action. It lists each tenant once;
`tenant_applications` is reserved for optional add-on subscriptions.

The workspace console groups Clients, Projects, and Documents under Core;
Staff under Settings; and Invoices and Add-ons under Billing. Overview
summarizes tenant-owned records and access. The Staff page displays each role
and its effective tenant permissions.
Each non-owner member links to a dedicated access page, and workspace Owners
can open a separate role-management page from the table. That page assigns a
member's role and configures the selected role's permissions for core resources;
role permission changes affect every member with that role. Owners can also
create staff accounts and assign an Administrator, Manager, or Support role.
Clients, Projects, Documents, and Billing are core areas provisioned for every tenant and are not
part of a subscription. Their
navigation and overview metrics appear only when the current member has the
matching read permission, and their write forms remain hidden for read-only
members. Every project belongs to a client from the same tenant, and project
forms require a client selection. Document create and edit forms require both a
title and description.
Core resources do not appear in the Add-ons catalog. That page derives optional
product availability from the workspace's purchased subscription. Designer
includes Approval Workflows and Priority Support, while Developer includes Feature
Flags and Priority Support. Analytics is active only for Developer. All four
add-ons are subscription-only and do not add permissions or role grants. Active
add-ons are listed under Paid in the sidebar, update after checkout, and open
workspace-scoped demonstration pages with add-on-specific descriptions.
Owners and Administrators can complete a fake purchase for an unavailable
add-on. The server activates the subscription and generates a paid demo
invoice; invoices cannot be entered manually from the SPA or API.
