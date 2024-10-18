use loco_rs::{controller::bad_request, prelude::*};
use serde::{Deserialize, Serialize};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    mailers::auth::AuthMailer,
    models::{
        _entities::users,
        users::{LoginParams, RegisterParams},
    },
    views::auth::UserSession,
};
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct VerifyParams {
    pub token: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ForgotParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ResetParams {
    pub token: String,
    pub password: String,
}

/// Register new user
///
/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user
#[utoipa::path(post, tag = "auth", request_body = RegisterParams, path = "/api/auth/register", responses((status = 200, body = UserSession)))]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    let user = match res {
        Ok(user) => user,
        Err(err) => {
            let msg = "could not register user";

            tracing::info!(message = err.to_string(), user_email = &params.email, msg,);
            return bad_request(msg);
        }
    };

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;

    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, &jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;
    format::json(UserSession::new(&user, &token))
}

/// Verify registered user
///
/// Verify register user. if the user not verified his email, he can't login to
/// the system.
#[utoipa::path(post, tag = "auth", request_body = VerifyParams, path = "/api/auth/verify", responses((status = 200)))]
async fn verify(
    State(ctx): State<AppContext>,
    Json(params): Json<VerifyParams>,
) -> Result<Response> {
    let user = users::Model::find_by_verification_token(&ctx.db, &params.token).await?;

    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        let active_model = user.into_active_model();
        let user = active_model.verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified");
    }

    format::empty_json()
}

/// Forgot password
///
/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for for security reasons (not exposing users DB
/// list).
#[utoipa::path(post, tag = "auth", request_body = ForgotParams, path = "/api/auth/forgot", responses((status = 200)))]
async fn forgot(
    State(ctx): State<AppContext>,
    Json(params): Json<ForgotParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return format::empty_json();
    };

    let user = user
        .into_active_model()
        .set_forgot_password_sent(&ctx.db)
        .await?;

    AuthMailer::forgot_password(&ctx, &user).await?;

    format::empty_json()
}

/// Reset password
///
/// reset user password by the given parameters
#[utoipa::path(post, tag = "auth", request_body = ResetParams, path = "/api/auth/reset", responses((status = 200)))]
async fn reset(State(ctx): State<AppContext>, Json(params): Json<ResetParams>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_reset_token(&ctx.db, &params.token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::info!("reset token not found");

        return format::empty_json();
    };
    user.into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    format::empty_json()
}

/// Login
///
/// Creates a user login and returns a token
#[utoipa::path(post, tag = "auth", request_body = LoginParams, path = "/api/auth/login", responses((status = 200, body = UserSession)))]
async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    let user = users::Model::find_by_email(&ctx.db, &params.email).await?;

    let valid = user.verify_password(&params.password);

    if !valid {
        return unauthorized("unauthorized!");
    }

    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, &jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(UserSession::new(&user, &token))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/register", post(register))
        .add("/verify", post(verify))
        .add("/login", post(login))
        .add("/forgot", post(forgot))
        .add("/reset", post(reset))
}

pub fn api_routes() -> OpenApiRouter<AppContext> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(verify))
        .routes(routes!(login))
        .routes(routes!(forgot))
        .routes(routes!(reset))
}
