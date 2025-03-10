#[cfg(feature = "auth_jwt")]
pub mod jwt;
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
pub mod openapi;
