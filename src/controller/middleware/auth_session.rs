//! Server-side session middleware.
//!
//! [`Session`](crate::auth::session::Session) changes are collected while a
//! handler runs and returned as response parts, which are synchronous. A
//! server-side store (`auth.session.store: database | redis`) is not, so this
//! middleware gives each request a shared session state, and writes the
//! pending store changes *before* the response goes out. If the store write
//! fails the client gets a `500` and none of the session cookies, never a
//! cookie that points at nothing.
//!
//! It is part of [`default_middleware_stack`](super::default_middleware_stack)
//! and only active with a server-side store. An app that builds its own stack
//! in [`Hooks::middlewares`](crate::app::Hooks::middlewares) must keep it.

use std::sync::Arc;

use axum::{
    extract::Request,
    middleware::{from_fn, Next},
    response::{IntoResponse, Response},
    Router as AXRouter,
};
use serde::Serialize;

use crate::{
    app::AppContext,
    auth::session::{Handled, Pending, Runtime},
    controller::middleware::MiddlewareLayer,
    Error, Result,
};

/// The `auth_session` middleware.
#[derive(Debug, Clone, Serialize)]
pub struct AuthSession {
    /// Whether `auth.session.store` is server-side.
    pub enable: bool,
}

impl AuthSession {
    /// Enabled when the configured session store is server-side.
    #[must_use]
    pub fn new(ctx: &AppContext) -> Self {
        Self {
            enable: ctx
                .config
                .auth
                .as_ref()
                .and_then(|a| a.session.as_ref())
                .is_some_and(|s| s.store.is_server_side()),
        }
    }
}

impl MiddlewareLayer for AuthSession {
    fn name(&self) -> &'static str {
        "auth_session"
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn config(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    fn apply(&self, app: AXRouter<AppContext>) -> Result<AXRouter<AppContext>> {
        Ok(app.layer(from_fn(run)))
    }
}

async fn run(mut req: Request, next: Next) -> Response {
    let runtime = Arc::new(Runtime::default());
    req.extensions_mut().insert(runtime.clone());
    let mut res = next.run(req).await;

    if let Some(Pending { store, ops }) = res.extensions_mut().remove::<Pending>()
        && let Err(err) = store.apply(ops).await
    {
        tracing::error!(error = %err, "could not write the session; dropping its cookies");
        return Error::Message("session store unavailable".to_string()).into_response();
    }

    // A session re-established from a remember-me cookie needs its new
    // cookies sent even when the handler never touched the session.
    if res.extensions().get::<Handled>().is_none()
        && let Some(session) = runtime.restored()
        && let Ok(jar) = session.into_jar()
    {
        return (jar, res).into_response();
    }
    res
}
