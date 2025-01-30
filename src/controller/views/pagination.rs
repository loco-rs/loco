use serde::{Deserialize, Serialize};

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
    #[serde(rename(serialize = "total_items"))]
    pub total_items: u64,
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
