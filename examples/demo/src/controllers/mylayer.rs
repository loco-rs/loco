#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::controllers::middlewares::handlers;

async fn user() -> Result<Response> {
    format::json("Hello, user!")
}

async fn admin() -> Result<Response> {
    format::json("Hello, admin!")
}

async fn echo() -> Result<Response> {
    format::json("Hello, World!")
}

pub fn routes(ctx: AppContext) -> Routes<AppContext> {
    Routes::new()
        .prefix("mylayer")
        // Only users with the RoleName::Admin can access this route
        .add(
            "/admin",
            get(admin).layer(handlers::admin::AdminHandlerLayer::new(ctx.clone())),
        )
        // Only users with the RoleName::User can access this route
        .add(
            "/user",
            get(user).layer(handlers::user::UserHandlerLayer::new(ctx.clone())),
        )
        .add("/echo", get(echo))
}
