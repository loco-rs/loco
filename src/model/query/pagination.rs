use crate::Result as LocoResult;
use sea_orm::{prelude::*, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

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
/// use loco_rs::prelude::{model::query::*, *};
///
/// #[derive(Debug, Deserialize)]
/// pub struct ListQueryParams {
///     pub title: Option<String>,
///     pub content: Option<String>,
///     #[serde(flatten)]
///     pub pagination: pagination::PaginationQuery,
/// }
///````
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

/// Deserialize pagination filter from string to u64 following a bug in `serde_urlencoded`.
fn deserialize_pagination_filter<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

/// /// Structure representing paginated response with rows and pagination information.
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

/// Paginate function for fetching paginated data from the database.
///
/// # Example
///
/// ```
/// use loco_rs::tests_cfg::db::*;
/// use sea_orm::EntityTrait;
/// use loco_rs::model::query::pagination;
/// use loco_rs::{prelude::model::*};
/// pub async fn data() {
///   let db = dummy_connection().await;
///   let entity = test_db::Entity::find();
///   let pagination_filter_query = query::pagination::PaginationQuery {
///        page_size: 1,
///        page: 1,
///    };
///   let res = pagination::paginate(&db, entity, None, &pagination_filter_query).await;
/// }
/// ```
/// # Errors
///
/// This function may return a `LocoResult` indicating any errors that occur during pagination.
pub async fn paginate<E>(
    db: &DatabaseConnection,
    entity: Select<E>,
    filters: Option<Condition>,
    pagination_query: &PaginationQuery,
) -> LocoResult<PaginatedResponse<E::Model>>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
{
    let page = if pagination_query.page <= 1 {
        0
    } else {
        pagination_query.page - 1
    };
    let entity = if let Some(filter) = filters {
        entity.filter(filter)
    } else {
        entity
    };

    let query = entity.paginate(db, pagination_query.page_size);
    let page_count = query.num_pages().await?;
    let rows: Vec<<E as EntityTrait>::Model> = query.fetch_page(page).await?;

    let paginated_response = PaginatedResponse {
        rows,
        info: PaginatedInfoResponse {
            page: pagination_query.page,
            page_size: pagination_query.page_size,
            total_pages: page_count,
        },
    };

    Ok(paginated_response)
}
