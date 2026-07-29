# reference_spa — golden reference stack (Loco 1.0 flagship frontend)

This app is the **golden reference** for Loco 1.0's flagship frontend: a **React SPA typed against
the Rust backend via `ts-rs`** (no OpenAPI). It is the exact target the rebuilt generator (workstream
2) will reproduce. See `docs/superpowers/audits/2026-07-04-dark-areas/DARK-AREAS-AUDIT.md` and
`docs/superpowers/specs/2026-07-04-golden-reference-stack-design.md`.

## Stack
- **Backend:** Loco/axum JSON API under `/api`; sea-orm 2.0; JWT auth.
- **Typed contract:** Rust DTOs (`src/dtos/`) `#[derive(ts_rs::TS)]` → committed TS bindings in
  `frontend/src/bindings/`. No OpenAPI; the DTO *is* the source of truth for both sides.
- **Frontend:** Vite + React 19 + react-router v8 + TanStack Query v5 (`frontend/`). A typed fetch
  client (`src/api/client.ts`) + Query hooks (`src/api/posts.ts`) consume the bindings.

## Run

### Dev (two processes)
```bash
# terminal 1 — backend on :5150
cargo run --bin reference_spa-cli -- start
# terminal 2 — Vite dev server on :5173 (proxies /api -> :5150)
cd frontend && pnpm install && pnpm dev
```
Open http://localhost:5173. Register a user (`POST /api/auth/register`), then log in.

### Prod (single origin, one binary)
```bash
cd frontend && pnpm build      # -> frontend/dist
cd .. && cargo run --bin reference_spa-cli -- start
```
Loco serves the built SPA at `/` (with SPA fallback) and the API at `/api` on :5150.

### Regenerate TS bindings after changing a DTO
```bash
cargo test --lib export_bindings   # rewrites frontend/src/bindings/*.ts
```
Bindings are committed to VCS so the frontend typechecks without a Rust build; CI should assert
regeneration produces no diff.

## Type conventions (locked)
| Rust | TS | how |
|---|---|---|
| `i64` (ids/FKs) | `number` | `#[ts(type = "number")]` |
| `Decimal` | `string` | `#[ts(type = "string")]` |
| `DateTimeWithTimeZone` | `string` | `#[ts(type = "string")]` |
| `Option<DateTime…>` | `string \| null` | `#[ts(type = "string \| null")]` (ts-rs replaces the type verbatim) |
| enum (serde snake_case) | string-literal union | derived |
| `Page<T>` | real TS generic | derived |

## What the generator emits (workstream-2 map)

**Emitted once per app (`loco new`):**
- `frontend/` scaffold (Vite config with `/api` proxy + `dist` outDir, router, `QueryClientProvider`).
- `frontend/src/api/client.ts` (typed fetch: Bearer, 204, 401→login, `ApiClientError`).
- `frontend/src/auth/` (`token.ts`, `Login.tsx`, `RequireAuth.tsx`).
- `src/dtos/common.rs` (`Page<T>`, `ApiError`), the bindings export step, `pub mod dtos;`.

**Emitted per resource (`generate scaffold <Name> <fields>`):**
- `src/dtos/<name>.rs` — DTOs with `#[derive(TS)]` + the convention attributes above +
  `From<Model>` (one column decision drives the migration column, the entity type, the DTO field,
  the `ts(type=…)` override, and the form input).
- `src/controllers/<name>.rs` — JSON CRUD returning the DTO shapes, `ApiError` on failure,
  `auth::JWT` guard.
- `frontend/src/api/<name>.ts` — TanStack Query hooks (list/detail/create/update/remove + keys).
- `frontend/src/pages/<name>/` — List/Show/New/Edit typed by the bindings.

**Generator watch-outs found here:** ts-rs `export_to` is relative to the `./bindings` default base
(pin `TS_RS_EXPORT_DIR`); `Option<T>` type overrides must include `| null` explicitly.
