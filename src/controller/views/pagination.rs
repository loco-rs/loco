use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

#[deprecated(
    since = "0.3.2",
    note = "reshape pagination functionality by moving under models. read more https://loco.rs/docs/the-app/pagination"
)]
pub trait PaginationResponseTrait {
    type Model: EntityTrait;
    type ResponseType;

    fn list(models: Vec<<Self::Model as EntityTrait>::Model>) -> Vec<Self::ResponseType>
    where
        Self: Sized;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pager<T> {
    #[serde(rename(serialize = "results"))]
    pub results: T,

    #[serde(rename(serialize = "pagination"))]
    pub info: PagerMeta,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagerMeta {
    #[serde(rename(serialize = "page"))]
    pub page: u64,
    #[serde(rename(serialize = "page_size"))]
    pub page_size: u64,
    #[serde(rename(serialize = "total_pages"))]
    pub total_pages: u64,
}

impl<T> Pager<T> {
    #[must_use]
    pub const fn new(results: T, meta: PagerMeta) -> Self {
        Self {
            results,
            info: meta,
        }
    }
}
