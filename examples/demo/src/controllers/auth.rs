use crate::mailers::auth::AuthMailer;
use crate::models::users::{self, LoginParams, RegisterParams};
use crate::views::auth::LoginResponse;
use axum::{extract::State, routing::post, Json};
use framework::{
    app::AppContext,
    controller::{format, unauthorized, Routes},
    Result,
};

async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Json<()>> {
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    let user = match res {
        Ok(user) => user,
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );
            return format::json(());
        }
    };

    // TODO:: send website base uri
    AuthMailer::send_welcome(&ctx, &user.email).await.unwrap();

    format::json(())
}

async fn login(
    State(ctx): State<AppContext>,
    Json(params): Json<LoginParams>,
) -> Result<Json<LoginResponse>> {
    let user = users::Model::find_by_email(&ctx.db, &params.email).await?;

    let valid = user.verify_password(&params.password)?;

    if !valid {
        return unauthorized("unauthorized access");
    }

    let token = user
        .generate_jwt(&ctx.config.auth.secret, &ctx.config.auth.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(LoginResponse::new(&user, &token))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register))
        .add("/login", post(login))
}
