use loco_rs::model::query::PageResponse;
use ts_rs::TS;

/// A page of results: the items, plus the framework's pagination metadata
/// flattened alongside them.
///
/// The field names are exactly `PagerMeta`'s (`page`, `page_size`,
/// `total_pages`, `total_items`), so an app has one pagination vocabulary
/// whether a handler answers with this typed envelope or with the framework's
/// own `Pager`. Build it with [`Page::from_query`] rather than by hand — that
/// is what keeps the two in step.
#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Page<T: TS> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub page: u64,
    #[ts(type = "number")]
    pub page_size: u64,
    #[ts(type = "number")]
    pub total_pages: u64,
    #[ts(type = "number")]
    pub total_items: u64,
}

impl<T: TS> Page<T> {
    /// Build a page from a paginated query (`query::paginate`,
    /// `query::fetch_page`), mapping each model onto its DTO.
    pub fn from_query<M>(res: PageResponse<M>) -> Self
    where
        T: From<M>,
    {
        Self {
            items: res.page.into_iter().map(T::from).collect(),
            page: res.meta.page,
            page_size: res.meta.page_size,
            total_pages: res.meta.total_pages,
            total_items: res.meta.total_items,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[ts(type = "unknown")]
    pub details: Option<serde_json::Value>,
}
