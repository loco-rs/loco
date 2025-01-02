mod middlewares;
#[cfg(any(
    feature = "openapi_swagger",
    feature = "openapi_redoc",
    feature = "openapi_scalar"
))]
mod openapi;
