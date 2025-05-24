use sea_orm::{prelude::*, Condition, DatabaseConnection, EntityTrait, QueryFilter, SelectorTrait};
use serde::{Deserialize, Serialize};

/// Set the default pagination page size.
const fn default_page_size() -> u64 {
    25
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

impl PaginationQuery {
    #[must_use]
    pub fn page(page: u64) -> Self {
        Self {
            page,
            ..Default::default()
        }
    }
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

#[derive(Debug)]
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub total_pages: u64,
    pub total_items: u64,
}

use crate::Result as LocoResult;

/// Paginate function for fetching paginated data from the database.
///
/// # Examples
///
/// Without conditions
/// ```
/// use loco_rs::tests_cfg::db;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = db::dummy_connection().await;
///     let pagination_query = query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     
///     let res = query::paginate(&db, db::test_db::Entity::find(), None, &pagination_query).await;
/// }
/// ````
/// With conditions
/// ```
/// use loco_rs::tests_cfg::db;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = db::dummy_connection().await;
///     let pagination_query = query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     let condition = query::condition().contains(db::test_db::Column::Name, "loco").build();
///     let res = query::paginate(&db, db::test_db::Entity::find(), Some(condition), &pagination_query).await;
/// }
/// ````
/// With Order By
/// ```
/// use loco_rs::tests_cfg::db;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait, sea_query::Order, QueryOrder};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = db::dummy_connection().await;
///     let pagination_query = query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     
///     let condition = query::condition().contains(db::test_db::Column::Name, "loco").build();
///     let entity = db::test_db::Entity::find().order_by(db::test_db::Column::Name, Order::Desc);
///     let res = query::paginate(&db, entity, Some(condition), &pagination_query).await;
/// }
/// ````
///
/// # Errors
///
/// Returns a `LocoResult` indicating any errors that occur
/// during pagination.
pub async fn paginate<E>(
    db: &DatabaseConnection,
    entity: Select<E>,
    condition: Option<Condition>,
    pagination_query: &PaginationQuery,
) -> LocoResult<PageResponse<E::Model>>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
{
    let page = pagination_query.page.saturating_sub(1);
    let entity = if let Some(condition) = condition {
        entity.filter(condition)
    } else {
        entity
    };

    let query = entity.paginate(db, pagination_query.page_size);
    let total_pages_and_items = query.num_items_and_pages().await?;
    let page: Vec<<E as EntityTrait>::Model> = query.fetch_page(page).await?;

    let paginated_response = PageResponse {
        page,
        total_pages: total_pages_and_items.number_of_pages,
        total_items: total_pages_and_items.number_of_items,
    };

    Ok(paginated_response)
}

/// Fetching a page from a selector.
///
/// # Examples
///
/// From Entity
/// ```
/// use loco_rs::tests_cfg::db;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = db::dummy_connection().await;
///     let pagination_query = query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     let res = query::fetch_page(&db, db::test_db::Entity::find(), &query::PaginationQuery::page(2)).await;
/// }
/// ``````
///
/// # Errors
///
/// Returns a `LocoResult` indicating any errors that occur
/// during the fetch.
pub async fn fetch_page<'db, C, S>(
    db: &'db C,
    selector: S,
    pagination_query: &PaginationQuery,
) -> LocoResult<PageResponse<<<S as PaginatorTrait<'db, C>>::Selector as SelectorTrait>::Item>>
where
    C: ConnectionTrait + Sync,
    S: PaginatorTrait<'db, C> + Send,
{
    let page = pagination_query.page.saturating_sub(1);

    let query = selector.paginate(db, pagination_query.page_size);
    let total_pages_and_items = query.num_items_and_pages().await?;
    let page = query.fetch_page(page).await?;

    Ok(PageResponse {
        page,
        total_pages: total_pages_and_items.number_of_pages,
        total_items: total_pages_and_items.number_of_items,
    })
}
