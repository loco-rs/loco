use loco_rs::prelude::*;

use crate::{models::_entities::users, views::user::CurrentResponse};

async fn current(
    auth: auth::JWTWithUser<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::json(CurrentResponse::new(&auth.user))
}

async fn current_from_cookie(
    auth: auth::JWTCookieWithUser<users::Model>,
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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current", get(current))
        .add("/current_api_key", get(current_by_api_key))
        .add("/current_from_cookie", get(current_from_cookie))
}
