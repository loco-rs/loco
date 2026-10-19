use loco_rs::prelude::*;

use crate::models::users;

/// Mark a user's address verified.
///
/// Each failure maps to the status that describes it. `Error::string(...)`
/// would render every one of these as a 500 while still compiling and
/// passing clippy — the framework's typed errors are what make the status
/// correct.
#[debug_handler]
async fn verify(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    // ModelError::EntityNotFound converts into Error::NotFound, so `?` already
    // yields a 404 here; being explicit keeps the mapping readable.
    let user = users::Model::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(|_| Error::NotFound)?;

    if user.email_verified_at.is_some() {
        return bad_request("user is already verified");
    }

    user.into_active_model().verified(&ctx.db).await?;

    format::empty_json()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/verification")
        .add("/{pid}", post(verify))
}
