use serde::{Deserialize, Serialize};

#[cfg_attr(
    any(
        feature = "openapi_swagger",
        feature = "openapi_redoc",
        feature = "openapi_scalar"
    ),
    derive(utoipa::ToSchema, utoipa::IntoParams)
)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Pager<T: utoipa::ToSchema> {
    #[serde(rename(serialize = "results"))]
    pub results: T,

    #[serde(rename(serialize = "pagination"))]
    pub info: PagerMeta,
}

#[cfg_attr(
    any(
        feature = "openapi_swagger",
        feature = "openapi_redoc",
        feature = "openapi_scalar"
    ),
    derive(utoipa::ToSchema, utoipa::IntoParams)
)]
#[derive(Debug, Deserialize, Serialize)]
pub struct PagerMeta {
    #[serde(rename(serialize = "page"))]
    pub page: u64,
    #[serde(rename(serialize = "page_size"))]
    pub page_size: u64,
    #[serde(rename(serialize = "total_pages"))]
    pub total_pages: u64,
    #[serde(rename(serialize = "total_items"))]
    pub total_items: u64,
}

impl<T: utoipa::ToSchema> Pager<T> {
    #[must_use]
    pub const fn new(results: T, meta: PagerMeta) -> Self {
        Self {
            results,
            info: meta,
        }
    }
}
