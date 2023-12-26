use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

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
    pub pagination: PagerMeta,
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
            pagination: meta,
        }
    }
}
