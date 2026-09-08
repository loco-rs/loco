use crate::{app::AppContext, Error};
use axum::{extract::FromRequestParts, http::request::Parts};
use std::any::Any;

/// An extractor that streamlines the process of getting static Data from the `DiContainer`.
pub struct SharedStore<T>(pub T);

impl<T> FromRequestParts<AppContext> for SharedStore<T>
where
    T: Any + Clone + Send + Sync + 'static,
{
    type Rejection = Error;

    // A `DiContainer` lookup, no I/O — see `extractor/auth.rs` for why the
    // `async fn` shape stays anyway.
    #[allow(clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        _: &mut Parts,
        state: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        let instance = state.shared_store.get::<T>().ok_or_else(|| {
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
