use serde::{Deserialize, Serialize};

pub mod dsl;
pub mod exec;

/// Set the default pagination page size.
const fn default_page_size() -> u64 {
    10
}

/// Set the default pagination page.
const fn default_page() -> u64 {
    1
}

/// Structure representing the pagination query parameters.
/// This struct allows to get the struct parameters from the query parameters.
///
/// # Example
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use loco_rs::prelude::model::*;
///
/// #[derive(Debug, Deserialize)]
/// pub struct ListQueryParams {
///     pub title: Option<String>,
///     pub content: Option<String>,
///     #[serde(flatten)]
///     pub pagination: query::PaginationQuery,
/// }
/// ````
#[derive(Debug, Deserialize, Serialize)]
pub struct PaginationQuery {
    #[serde(
        default = "default_page_size",
        rename = "page_size",
        deserialize_with = "deserialize_pagination_filter"
    )]
    pub page_size: u64,
    #[serde(
        default = "default_page",
        rename = "page",
        deserialize_with = "deserialize_pagination_filter"
    )]
    pub page: u64,
}

/// Default implementation for `PaginationQuery`.
impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
            page: default_page(),
        }
    }
}

/// Deserialize pagination filter from string to u64 following a bug in
/// `serde_urlencoded`.
fn deserialize_pagination_filter<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

/// /// Structure representing paginated response with rows and pagination
/// information.
#[derive(Debug)]
pub struct PaginatedResponse<T> {
    pub rows: Vec<T>,
    pub info: PaginatedInfoResponse,
}

/// Structure representing pagination information in a paginated response.
#[derive(Debug, Deserialize, Serialize)]
pub struct PaginatedInfoResponse {
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}
