# Controllers

A controller is a regular Rust module that exposes a `routes()` method which we then use to compose into the root app router.

## Adding Controllers

First add a controller in `controllers/users.rs`

```rust
use axum::{extract::State, routing::get, Json};
use loco_rs::{
    app::AppContext,
    controller::{format, middleware, Routes},
    Result,
};

use crate::{models::_entities::users, views::user::CurrentResponse};

async fn current(
    auth: middleware::auth::Auth,
    State(ctx): State<AppContext>,
) -> Result<Json<CurrentResponse>> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

pub fn routes() -> Routes {
    Routes::new().prefix("user").add("/current", get(current))
}
```

Update the `mod` file: `controllers/mod.rs`:

```rust
pub mod user;
```

And register the routes in your main `app.rs` file:

```rust
// ...
impl Hooks for App {
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::auth::routes())
            .add_route(controllers::user::routes()) // <--- add this
    }
```
