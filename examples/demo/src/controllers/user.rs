use loco_rs::prelude::*;

use crate::{
    models::{_entities::users, roles},
    views::user::{CurrentResponse, UserResponse},
};

async fn current(
    auth: auth::JWTWithUser<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(CurrentResponse::new(&auth.user))
}

async fn current_by_api_key(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(CurrentResponse::new(&auth.user))
}

async fn convert_to_admin(
    auth: auth::JWTWithUser<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let roles = roles::Model::add_user_to_admin_role(&ctx.db, &auth.user).await?;
    format::json(UserResponse::new(&auth.user, &roles))
}

async fn convert_to_user(
    auth: auth::JWTWithUser<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let roles = roles::Model::add_user_to_user_role(&ctx.db, &auth.user).await?;
    format::json(UserResponse::new(&auth.user, &roles))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current", get(current))
        .add("/current_api_key", get(current_by_api_key))
        .add("/convert/admin", post(convert_to_admin))
        .add("/convert/user", post(convert_to_user))
}
