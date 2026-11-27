//! Signing in from a test, Devise's `sign_in user`.
//!
//! ```rust,ignore
//! use loco_rs::testing::prelude::*;
//!
//! #[tokio::test]
//! async fn shows_the_dashboard() {
//!     request::<App, _, _>(|mut request, ctx| async move {
//!         let user = users::Model::find_by_email(&ctx.db, "user1@example.com").await.unwrap();
//!         sign_in(&mut request, &ctx, user.pid.to_string()).await.unwrap();
//!         request.get("/dashboard").await.assert_status_ok();
//!     })
//!     .await;
//! }
//! ```

use axum_test::TestServer;

use crate::{app::AppContext, auth::session, Result};

/// Signs `server` in as the user whose session id is `user_id`.
///
/// `user_id` is what
/// [`SessionUser::find_by_session_id`](crate::auth::session::SessionUser::find_by_session_id)
/// receives. Later requests from `server` carry a session for that user, on
/// whichever `auth.session.store` the app uses. Sign out again with
/// `server.clear_cookies()`.
///
/// # Errors
///
/// When the cookie key is unavailable or the server-side session store
/// cannot be written.
pub async fn sign_in(
    server: &mut TestServer,
    ctx: &AppContext,
    user_id: impl Into<String> + Send,
) -> Result<()> {
    for cookie in session::sign_in_cookies(ctx, user_id).await? {
        server.add_cookie(cookie);
    }
    Ok(())
}
