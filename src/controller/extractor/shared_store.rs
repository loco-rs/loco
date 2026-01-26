use crate::app;
use crate::Error;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use std::any::Any;
use std::sync::Arc;

/// An extractor that streamlines the process of getting static Data from the `DiContainer`.
pub struct SharedStore<T>(pub T);

impl<T, S> FromRequestParts<S> for SharedStore<T>
where
    T: Any + Clone + Send + Sync + 'static,
    S: Send + Sync,
    Arc<app::SharedStore>: FromRef<S>,
{
    type Rejection = Error;

    async fn from_request_parts(_: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let shared_store: Arc<app::SharedStore> = FromRef::from_ref(state);
        let instance = shared_store.get::<T>().ok_or_else(|| {
            let type_name = std::any::type_name::<T>();
            tracing::error!(
                "Could not find service of type `{}` in shared store",
                type_name
            );
            Error::InternalServerError
        })?;

        Ok(Self(instance))
    }
}
