pub mod driver;

use crate::request_context::driver::Driver;
use axum_extra::extract::cookie::Key;

#[derive(thiserror::Error, Debug)]
pub enum RequestContextError {
    #[error("Tower Session error")]
    TowerSessionError,

    #[error("Cookie Session error")]
    CookieSessionError,

    // Retrieve data from cookie error
    #[error("Cookie Session Data error: {0}")]
    CookieSessionDataError(#[from] serde_json::Error),

    // Retrieve data from tower session error
    #[error("Tower Session Data error: {0}")]
    TowerSessionDataError(#[from] tower_sessions::session::Error),

    // Convert Signed private cookie jar error
    #[error("Signed private cookie jar error: {0}")]
    SignedPrivateCookieJarError(#[from] driver::cookie::SignedPrivateCookieJarError),
}

#[derive(Debug, Clone)]
pub struct RequestContextStore {
    signed_key: Key,
    private_key: Key,
}

impl RequestContextStore {
    #[must_use]
    pub fn new(signed_key: Key, private_key: Key) -> Self {
        Self {
            signed_key,
            private_key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: uuid::Uuid,
    pub driver: Driver,
}

// #[async_trait]
// impl<S> FromRequestParts<S> for RequestContext
// where
//     AppContext: FromRef<S>,
//     S: Send + Sync,
// {
//     type Rejection = prelude::Error;
//
//     async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
//         let ctx: AppContext = AppContext::from_ref(state);
//         let driver = match ctx.config.request_context {
//             crate::config::RequestContext::Cookie {
//                 // private_key,
//                 // signed_key,
//                 ..
//             } => {
//
//                 let jar = PrivateCookieJar::from_headers(&req.headers, Key::generate());
//                 Driver::PrivateCookieJar(jar)
//             }
//             crate::config::RequestContext::Tower => {
//                 let session = Session::from_request_parts(req, state)
//                     .await
//                     .map_err(|e| {
//                         tracing::error!(?e, "Failed to create tower session");
//                         RequestContextError::TowerSessionError
//                     })?;
//                 Driver::TowerSession(session)
//             }
//         };
//
//         let request_id = req
//             .extensions
//             .get::<uuid::Uuid>()
//             .expect("Request Id should be created in parent middleware");
//
//         Ok(Self {
//             driver,
//             request_id: *request_id,
//         })
//     }
// }
