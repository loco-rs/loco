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

    fn from_request_parts(
        _: &mut Parts,
        state: &AppContext,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> {
        let instance = state.shared_store.get::<T>().map(Self).ok_or_else(|| {
            let type_name = std::any::type_name::<T>();
            tracing::error!(
                "Could not find service of type `{}` in shared store",
                type_name
            );
            Error::InternalServerError
        });

        std::future::ready(instance)
    }
}
