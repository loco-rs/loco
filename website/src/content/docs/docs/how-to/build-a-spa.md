---
title: Build a typed React SPA
description: "Use Loco's clientside mode: a Vite + React + TanStack Query frontend whose TypeScript types are generated from your Rust DTOs by ts-rs, so a schema change breaks the frontend build instead of production."
sidebar:
  order: 18
---

**Goal:** build a single-page app against your Loco backend where the TypeScript types come from your Rust types — not from a hand-maintained copy that silently drifts.

This is what `loco new` produces when you pick **clientside** assets. If you already have an app, the layout below is what you need to add.

## 1. What you get

A clientside app is one Cargo project with a `frontend/` directory inside it:

```
myapp/
├── src/
│   ├── controllers/         # your JSON API
│   └── dtos/                # the wire types — the source of truth
├── frontend/
│   ├── package.json         # react 19, react-router, @tanstack/react-query
│   ├── vite.config.ts
│   └── src/
│       ├── main.tsx         # QueryClientProvider + RouterProvider
│       ├── routes.tsx       # the route table
│       ├── api/client.ts    # fetch wrapper: bearer token, error mapping
│       ├── bindings/        # GENERATED TypeScript — never edit by hand
│       ├── auth/            # token storage, Login, RequireAuth
│       └── pages/
└── config/
```

Two directories carry the whole idea: **`src/dtos/`** holds Rust types, and **`frontend/src/bindings/`** holds their TypeScript equivalents, generated.

## 2. The type pipeline

A DTO is a plain Rust struct that derives [`ts_rs::TS`](https://docs.rs/ts-rs):

```rust
use ts_rs::TS;

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PostDto {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub status: PostStatus,
    #[ts(type = "string")]
    pub price: Decimal,
    #[ts(type = "string | null")]
    pub published_at: Option<DateTimeWithTimeZone>,
}
```

`#[ts(type = "...")]` is how you pin the wire shape of a type `ts-rs` can't infer on its own — `i64` is a JavaScript `number`, and a `Decimal` crosses the wire as a `string` so it doesn't lose precision.

Keep DTOs separate from your Sea-ORM entities and convert at the edge:

```rust
impl From<crate::models::_entities::posts::Model> for PostDto {
    fn from(m: crate::models::_entities::posts::Model) -> Self {
        Self { id: m.id, title: m.title, status: PostStatus::from(m.status), .. }
    }
}
```

That `From` is the seam. Your database schema can change without changing your API, and when you *do* want the API to change, the compiler walks you through it.

### Regenerating the bindings

**`#[ts(export)]` generates a test.** The `.ts` files are written when you run:

```sh
cargo test
```

That is the whole command — there is no separate export step and no build script. Bindings are refreshed as a side effect of the test suite, which means CI regenerates them on every run and a stale binding shows up as a diff.

After changing a DTO, run `cargo test`, then rebuild the frontend. A field you removed in Rust is now a TypeScript compile error in every page that read it.

## 3. Scaffold a resource

With a `frontend/` present, `scaffold` is adaptive — it generates the backend *and* the frontend:

```sh
cargo loco generate scaffold post title:string content:text status:enum:draft,published
```

You get the usual model, migration, and controller, plus:

| File | What it is |
| --- | --- |
| `src/dtos/posts.rs` | `PostDto`, `CreatePost`, `UpdatePost`, enums — all `#[ts(export)]` |
| `frontend/src/api/posts.ts` | typed TanStack Query hooks: `useListPosts`, `usePost`, `useCreatePost`, `useUpdatePost`, `useRemovePost` |
| `frontend/src/pages/posts/` | `List`, `Show`, `New`, `Edit` |
| `frontend/src/routes.tsx` | imports and routes, injected at the `// scaffold:imports` and `// scaffold:routes` anchors |

Those two anchor comments must stay in `routes.tsx`. They are how the generator finds its place; if you delete them, the next scaffold fails with an explicit error rather than silently generating pages nothing routes to.

The generated hooks own their cache invalidation, so a create or delete refreshes the list without any wiring on your part:

```tsx
export function List() {
  const { data, isLoading, isError, error } = useListPosts();
  const removePost = useRemovePost();
  // ...
}
```

## 4. The development loop

Run the two servers side by side:

```sh
cargo loco start                 # :5150 — the API
cd frontend && pnpm install && pnpm dev   # :5173 — Vite, with HMR
```

Develop against **`http://localhost:5173`**. Vite proxies `/api` to the backend, so the browser sees one origin and there is no CORS to configure:

```ts
// frontend/vite.config.ts
server: { port: 5173, proxy: { '/api': 'http://localhost:5150' } }
```

## 5. Ship it

Build the frontend, then start the app:

```sh
cd frontend && pnpm build        # writes frontend/dist/
cargo loco start
```

Loco serves the bundle through the static middleware, already configured for you:

```yaml
server:
  middlewares:
    fallback:
      enable: false
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
```

The `fallback` key inside `static` is what makes client-side routing survive a hard refresh: a request for `/posts/42` matches no file, so `index.html` is served and React Router takes over.

:::caution
`must_exist: true` means the app **refuses to start until `frontend/dist` exists**. On a freshly generated clientside app, `cargo loco start` fails before your code runs — that is not a broken app, it is the missing frontend build. Run `pnpm build` once first. The same applies in CI and in your Dockerfile: build the frontend before starting the binary, or set `must_exist: false` and accept 404s until you do.
:::

To ship a single self-contained binary with the bundle compiled in, see [`embedded_assets`](/docs/how-to/serve-assets#6-embed-assets-into-the-binary-with-embedded_assets).

## 6. Authentication

`frontend/src/api/client.ts` attaches the JWT to every request and handles expiry centrally:

```ts
const token = getToken();
if (token) {
  headers["Authorization"] = `Bearer ${token}`;
}
// ...
if (res.status === 401) {
  clearToken();
  window.location.href = "/login";
}
```

Route protection is one component — `RequireAuth` renders an `<Outlet />` when a token is present and redirects otherwise:

```tsx
export function RequireAuth() {
  if (getToken() === null) {
    return <Navigate to="/login" replace />;
  }
  return <Outlet />;
}
```

Scaffolded routes are injected **inside** the `RequireAuth` branch of the route table, matching the backend: generated controllers require a JWT. See [JWT authentication](/docs/how-to/jwt-auth) for the server side.

The generated token store uses `localStorage`. That is the simplest thing that works for a getting-started app; if XSS-resistant storage matters for your threat model, move the token to an httpOnly cookie and switch the server to the cookie JWT location — see [JWT locations](/docs/how-to/jwt-locations).

## 7. A complete example

[`examples/reference_spa`](https://github.com/loco-rs/loco/tree/master/examples/reference_spa) in the Loco repository is a full working app built exactly this way — DTOs with enums and decimals, generated bindings, typed hooks, and the four scaffolded pages. It is the app `loco new` + `generate scaffold` reproduces, and it is exercised by the test suite, so it stays honest.

## Related

- [Serve static & SPA assets](/docs/how-to/serve-assets) — the static middleware in full
- [Add a controller](/docs/how-to/add-controller) — the API the SPA calls
- [Use the generators](/docs/how-to/use-generators) — every generator and flag
- [Deploy](/docs/how-to/deploy) — remember to build the frontend in your pipeline
