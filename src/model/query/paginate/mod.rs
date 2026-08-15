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

use crate::controller::views::pagination::PagerMeta;

/// A page of rows plus its metadata, as returned by [`paginate`] and
/// [`fetch_page`].
///
/// It is `Serialize`/`Deserialize` because returning it straight from a
/// handler (`format::json(res)`) is the documented shortest path — that had
/// never compiled outside the crate. Handlers that answer with a typed DTO
/// instead map it onto their own envelope; the generated scaffold's `Page<T>`
/// is exactly this shape flattened.
#[derive(Debug, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub page: Vec<T>,
    pub meta: PagerMeta,
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
    let entity = if let Some(condition) = condition {
        entity.filter(condition)
    } else {
        entity
    };

    fetch_page(db, entity, pagination_query).await
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

    // Clamp to at least 1 to avoid a divide-by-zero panic inside sea-orm's
    // paginator when a client sends `page_size=0` (see `PaginationQuery`).
    let page_size = pagination_query.page_size.max(1);

    let query = selector.paginate(db, page_size);
    let total_pages_and_items = query.num_items_and_pages().await?;
    let page = query.fetch_page(page).await?;

    Ok(PageResponse {
        page,
        meta: PagerMeta {
            page: pagination_query.page,
            page_size,
            total_pages: total_pages_and_items.number_of_pages,
            total_items: total_pages_and_items.number_of_items,
        },
    })
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use serde::Deserialize;

    use super::*;

    fn parse<T: serde::de::DeserializeOwned>(query: &str) -> T {
        let uri: axum::http::Uri = format!("http://localhost/?{query}")
            .parse()
            .expect("a valid URI");
        Query::try_from_uri(&uri)
            .unwrap_or_else(|err| panic!("`?{query}` should deserialize: {err}"))
            .0
    }

    /// The how-to shows `format::json(res)` straight from a handler and prints
    /// the body it produces. That only holds if `PageResponse` serializes at
    /// all -- it did not until it derived it -- and if the body keeps these
    /// exact keys, so pin them.
    #[test]
    fn serializes_to_page_and_meta() {
        let res = PageResponse {
            page: vec![1, 2, 3],
            meta: PagerMeta {
                page: 2,
                page_size: 3,
                total_pages: 4,
                total_items: 10,
            },
        };

        let json = serde_json::to_value(&res).expect("PageResponse serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "page": [1, 2, 3],
                "meta": {
                    "page": 2,
                    "page_size": 3,
                    "total_pages": 4,
                    "total_items": 10
                }
            })
        );

        let back: PageResponse<i32> =
            serde_json::from_value(json).expect("PageResponse reads its own output");
        assert_eq!(back.page, vec![1, 2, 3]);
        assert_eq!(back.meta.total_items, 10);
    }

    #[test]
    fn reads_page_and_page_size_from_a_query_string() {
        let q: PaginationQuery = parse("page=2&page_size=10");
        assert_eq!(q.page, 2);
        assert_eq!(q.page_size, 10);
    }

    #[test]
    fn falls_back_to_defaults_when_absent() {
        let q: PaginationQuery = parse("");
        assert_eq!(q.page, default_page());
        assert_eq!(q.page_size, default_page_size());
    }

    /// The documented way to add filters to a paginated endpoint -- and what a
    /// scaffolded `list` handler grows into -- is to `#[serde(flatten)]` this
    /// struct into the resource's own params. That combination is worth pinning:
    /// `flatten` makes serde buffer the query into a map first, and every field
    /// here is read through a `deserialize_with` that expects a string, so the
    /// two features have to agree for `?status=draft&page=2` to parse at all.
    #[test]
    fn survives_being_flattened_into_a_filter_struct() {
        #[derive(Debug, Deserialize)]
        struct ListParams {
            status: Option<String>,
            #[serde(flatten)]
            pagination: PaginationQuery,
        }

        let params: ListParams = parse("status=draft&page=3&page_size=5");
        assert_eq!(params.status.as_deref(), Some("draft"));
        assert_eq!(params.pagination.page, 3);
        assert_eq!(params.pagination.page_size, 5);

        // ...and the per-field defaults still apply through the flatten.
        let defaults: ListParams = parse("status=draft");
        assert_eq!(defaults.pagination.page, default_page());
        assert_eq!(defaults.pagination.page_size, default_page_size());
    }
}
