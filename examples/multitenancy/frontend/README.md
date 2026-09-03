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

The SPA includes registration, login, tenant/application selection, and
logout. Registration asks for the user's name, email, password, and tenant
name in separate rows; the tenant slug is generated automatically. It creates
the initial tenant and owner permissions, login uses Loco's JWT endpoint, and
`/api/auth/workspaces` returns only the active tenant subscriptions available
to the authenticated user.
