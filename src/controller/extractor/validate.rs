use axum::extract::{Form, FromRequest, Json, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::Error;

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonValidateWithMessage<T>(pub T);

impl<T, S> FromRequest<S> for JsonValidateWithMessage<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FormValidateWithMessage<T>(pub T);

impl<T, S> FromRequest<S> for FormValidateWithMessage<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonValidate<T>(pub T);

impl<T, S> FromRequest<S> for JsonValidate<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "request validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FormValidate<T>(pub T);

impl<T, S> FromRequest<S> for FormValidate<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate().map_err(|err| {
            tracing::debug!(err = ?err, "request validation error occurred");
            Error::BadRequest(String::new())
        })?;
        Ok(Self(value))
    }
}
