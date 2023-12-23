use loco_rs::prelude::*;

use crate::{models::_entities::users, views::user::CurrentResponse};

async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Json<CurrentResponse>> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

async fn current_by_api_key(
    auth: auth::ApiToken<users::Model>,
    State(_ctx): State<AppContext>,
) -> Result<Json<CurrentResponse>> {
    format::json(CurrentResponse::new(&auth.user))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("user")
        .add("/current", get(current))
        .add("/current_api_key", get(current_by_api_key))
}
