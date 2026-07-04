# Golden Reference Stack — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans to implement task-by-task. Steps use `- [ ]` checkboxes.

**Goal:** A working reference Loco app on the decided flagship stack (Loco JSON API + `ts-rs`
bindings + Vite/React/react-router/TanStack Query SPA + JWT auth), running dev + prod, that becomes
the generator's target output.

**Architecture:** Backend exposes `/api` JSON handlers returning DTOs; DTOs `#[derive(TS)]` export
`.ts` bindings into the frontend; a typed fetch client + TanStack Query hooks consume them; react-router
renders pages; JWT via `Authorization: Bearer` (localStorage). Dev: Vite proxies `/api`→:5150. Prod:
Loco serves `frontend/dist` with SPA fallback.

**Tech Stack:** Rust/axum/loco-rs (local via `LOCO_DEV_MODE_PATH`), sea-orm 2.0-rc, `ts-rs` 12; React
19, Vite, react-router v8, @tanstack/react-query v5, TypeScript, pnpm, Tailwind standalone CLI.

## Global Constraints
- Local commits only (Jondot does all push/PR/publish). Commit msgs end with the session trailer.
- Governor (Opus) reviews every diff, runs gates, commits when no agent mid-run. Parallel-git-safety:
  no concurrent state-changing git; implementers run sequentially; agents forbidden all git writes.
- Reference app lives OUTSIDE the loco workspace (workspace `exclude`s `examples/`, `starters/`); put
  it under `examples/reference-spa/` (excluded from the workspace) or a scratch path, TBD in Task 0.
- Type conventions: `i64`→TS `number`; `Decimal`→`string`; `DateTime`→`string`; `Option`→`T|null`.
- Every task ends green: `cargo check`+`clippy` (backend), `tsc --noEmit`+`vite build` (frontend),
  relevant smoke.

---

### Task 0: Toolchain + environment spike (de-risk before building) — ✅ DONE
**Outcome (2026-07-04):** GATE PASSED with one required pin.
- Generate app: `LOCO_DEV_MODE_PATH=/Users/jondot/projects/loco cargo run --manifest-path
  loco-new/Cargo.toml -- new -p <dir> -n <name> --db sqlite --bg async --assets clientside -a`
  → wires `loco-rs = { path = "/Users/jondot/projects/loco" }`.
- **REQUIRED PIN:** fresh apps resolve `sea-schema 0.18.1` which breaks `sea-orm 2.0.0-rc.41`
  (`Connection` gained a `Send` bound vs the crate's `?Send` impl → E0053). Fix per app:
  `cargo update -p sea-schema --precise 0.18.0`. With it, `cargo check` is clean (loco-rs compiles
  fine on stable 1.96.1). Also a **live base_template bug** (gitignored Cargo.lock → every `loco new`
  breaks); recommend pinning `sea-schema = "=0.18.0"` in the starter until Sea-ORM 2.0 stable.
- Vite + React + **react-router v8** (8.1.0) + TanStack Query v5 (5.10): `tsc` + `pnpm build` clean.
- Docker/Colima reachable (default socket; `DOCKER_HOST` unset is fine). `1.95.0` installed; no 1.94.
- Reference app location: `examples/reference-spa/` (workspace `exclude`s `examples/`).

### Task 1: Base app + Vite/React/router/Query skeleton
**Files:** Create `examples/reference-spa/**` (backend from `loco new` clientside), replace
`frontend/` with Vite scaffold; add react-router v8 + @tanstack/react-query.
- [ ] Generate base app; strip the rsbuild/splash `frontend/`.
- [ ] New `frontend/` — Vite React-TS, `react-router` route table with a placeholder Home, a
      `QueryClientProvider` in `main.tsx`, `vite.config.ts` proxy `/api`→`http://localhost:5150`.
- [ ] **Validate:** `pnpm build` clean; `cargo check` clean; `cargo loco start` serves and Vite dev
      loads Home. Commit.

### Task 2: `ts-rs` DTOs + bindings export step
**Files:** `src/dtos/mod.rs`, `src/dtos/posts.rs`; export mechanism (test or `src/bin/export_bindings.rs`);
`frontend/src/bindings/` (generated, committed).
**Interfaces produced:** `PostDto`, `CreatePost`, `UpdatePost`, `PostStatus`, `Attachment`, `Author`,
`Page<T>`, `ApiError` — all `#[derive(Serialize, Deserialize, TS)]` with the type-convention
attributes; a `posts` migration+entity so `From<Model> for PostDto` exists.
- [ ] Add `ts-rs` (features: `chrono-impl`, `serde-compat`, `serde-json-impl`) to the app Cargo.toml.
- [ ] Write the DTOs with `#[ts(export, export_to = "frontend/src/bindings/")]`; `i64` id/FK →
      `#[ts(type="number")]`, `Decimal`→`#[ts(type="string")]`, `Value`→`#[ts(type="unknown")]`.
- [ ] Wire the export step; run it → `frontend/src/bindings/*.ts` appear.
- [ ] **Validate:** re-running export yields no diff; `tsc --noEmit` sees the bindings; `cargo check`
      clean. Commit.

### Task 3: API controllers + `ApiError` envelope
**Files:** `src/controllers/posts.rs`, `src/controllers/mod.rs`; error responder alignment.
**Interfaces consumed:** DTOs from Task 2.
- [ ] `list/get/create/update/remove` under `/api/posts`, returning `Json<PostDto>` /
      `Json<Page<PostDto>>`; map `entities::posts::Model`→`PostDto`.
- [ ] Align Loco error responses to `ApiError { code, message, details }` (422 carries field errors).
- [ ] **Validate:** `cargo check`+`clippy`; hit endpoints with `curl` (seeded row) → correct JSON +
      error shape. Commit.

### Task 4: Typed fetch client + TanStack Query hooks
**Files:** `frontend/src/api/client.ts`, `frontend/src/api/posts.ts`.
**Interfaces consumed:** `bindings/*`.
- [ ] `client.ts`: `get/post/put/del` over `fetch`, `/api` prefix, `Authorization: Bearer` from token
      store, non-2xx → thrown `ApiError`, 401 → clear token + redirect `/login`.
- [ ] `posts.ts`: `useListPosts`, `usePost(id)`, `useCreatePost`, `useUpdatePost`, `useRemovePost`
      (typed by bindings, with cache invalidation).
- [ ] **Validate:** `tsc --noEmit` clean; hooks typecheck against bindings. Commit.

### Task 5: React pages
**Files:** `frontend/src/pages/posts/{List,Show,New,Edit}.tsx`, route table in `App.tsx`.
- [ ] List (table + create link), Show, New/Edit forms typed by `CreatePost`/`UpdatePost` (exercise
      the enum, a nullable field, the tagged-enum attachment). Error + loading states from Query.
- [ ] **Validate:** `tsc`+`vite build` clean; dev run: CRUD works against the API. Commit.

### Task 6: JWT auth flow + auth endpoint reshape
**Files:** `frontend/src/auth/{token.ts,RequireAuth.tsx,Login.tsx}`; backend auth controller reshape to
`ApiError`+DTO.
- [ ] Login form → `/api/auth/login` → `{token}` in localStorage; `RequireAuth` gates `/posts*`; 401
      handling verified. Reshape login/register responses to the envelope + DTO conventions.
- [ ] **Validate:** login → authed CRUD; logout/expired → redirect. `cargo check`+`tsc`+`vite build`.
      Commit.

### Task 7: End-to-end validation + freeze as reference
**Files:** `examples/reference-spa/README.md` (run instructions), a short smoke doc.
- [ ] Dev: two-process run, full smoke (login→list→create with rich types→see it; 401 redirect).
- [ ] Prod: `pnpm build` + `cargo loco start` serves SPA+API one origin; SPA fallback works.
- [ ] Document what's emitted-once vs per-resource (feeds workstream 2). Commit.

## Self-Review notes
- Spec coverage: Tasks map to spec sections (DTO/contract T2, API T3, client T4, pages T5, auth T6,
  serving T1/T7). Open questions #1–#5 resolved via defaults (bindings committed+CI check; router v7;
  pnpm+Tailwind standalone; localStorage Bearer; reshape auth).
- Risk: Task 0 is the gate — if the local loco app can't build under sea-orm 2.0-rc, resolve before
  Task 1. i64→number override must be applied consistently or the client/entity types diverge.
