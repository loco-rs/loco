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

The SPA includes registration, login, tenant/application selection, workspace
creation, and logout. Registration asks only for the user's name, email, and
password, then opens the workspace modal. Tenant slugs are generated from the
workspace name automatically. Login uses Loco's JWT endpoint, and
`/api/auth/workspaces` lists and creates workspaces for the authenticated user.
The authenticated navbar contains an organization-style workspace menu, with
`New workspace` as its final action. It lists each tenant once and derives the
available application contexts from active `tenant_applications` subscriptions.

The workspace console provides Overview, Documents, Billing, Members, and
Add-ons pages. Overview summarizes tenant-owned records and access. The
Members page displays each role and its effective application permissions.
Each non-owner member links to a dedicated access page, and workspace Owners
can open a separate role-management page from the table. That page assigns a
member's role and configures the selected role's permissions across active
workspace applications; role permission changes affect every member with that
role.
The Add-ons page excludes the core Documents and Billing applications. It shows
optional products such as Analytics and derives their availability from the
workspace's purchased subscription.
Documents and Billing use the selected tenant's matching subscription and hide
write forms when the current member has read-only access.
