use crate::models::_entities::users;
use crate::views::user::CurrentResponse;
use axum::{extract::State, routing::get, Json};
use framework::{
    app::AppContext,
    controller::middleware,
    controller::{format, Routes},
    Result,
};

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
