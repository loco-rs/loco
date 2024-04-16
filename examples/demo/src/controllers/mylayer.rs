#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::controllers::middleware;

async fn user() -> Result<Response> {
    format::json("Hello, user!")
}

async fn admin() -> Result<Response> {
    format::json("Hello, admin!")
}

async fn echo() -> Result<Response> {
    format::json("Hello, World!")
}

pub fn routes(ctx: AppContext) -> Routes {
    Routes::new()
        .prefix("mylayer")
        // Only users with the RoleName::Admin can access this route
        .add(
            "/admin",
            get(admin).layer(middleware::role::AdminHandlerLayer::new(ctx.clone())),
        )
        // Only users with the RoleName::User can access this route
        .add(
            "/user",
            get(user).layer(middleware::role::UserHandlerLayer::new(ctx.clone())),
        )
        .add("/echo", get(echo))
}
