// use sea_orm::{prelude::*, DatabaseConnection, EntityTrait, PaginatorTrait, Select, SelectorTrait};
use crate::{
    controller::views::pagination::{Pager, PagerMeta, PaginationResponseTrait},
    Result as LocoResult,
};
use sea_orm::{prelude::*, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;

/// Set the default pagination page size
const fn default_page_size() -> u64 {
    10
}

/// Set the default pagination page
const fn default_page() -> u64 {
    1
}

#[derive(Debug, Deserialize)]
pub struct PaginationFilter {
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

/// Parse the parameters to u64 following a bug in `serde_urlencoded`
fn deserialize_pagination_filter<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[derive(Debug)]
pub struct PaginatedResponse<T> {
    pub rows: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

/// Paginates a database query for a given entity, applying optional filters and pagination settings.
/// After paginate the db rows result sends to `PaginationResponseTrait` for prepare json response.
///
/// # Errors
/// when could not fetch the entity query
pub async fn view<R, E>(
    db: &DatabaseConnection,
    entity: Select<E>,
    filters: Option<Condition>,
    pagination_filter: &PaginationFilter,
) -> LocoResult<Pager<Vec<<R as PaginationResponseTrait>::ResponseType>>>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
    R: PaginationResponseTrait<Model = E>,
{
    let res = paginate::<R, E>(db, entity, filters, pagination_filter).await?;

    let res = Pager {
        results: R::list(res.rows),
        pagination: PagerMeta {
            page: res.page,
            page_size: res.page_size,
            total_pages: res.total_pages,
        },
    };

    Ok(res)
}

/// Paginates a database query for a given entity, applying optional filters and pagination settings.
///
/// # Errors
/// when could not fetch the entity query
pub async fn paginate<R, E>(
    db: &DatabaseConnection,
    entity: Select<E>,
    filters: Option<Condition>,
    pagination_filter: &PaginationFilter,
) -> LocoResult<PaginatedResponse<<E as EntityTrait>::Model>>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
    R: PaginationResponseTrait<Model = E>,
{
    let page = if pagination_filter.page <= 1 {
        0
    } else {
        pagination_filter.page - 1
    };
    let entity = if let Some(filter) = filters {
        entity.filter(filter)
    } else {
        entity
    };

    let query = entity.paginate(db, pagination_filter.page_size);
    let page_count = query.num_pages().await?;
    let rows: Vec<<E as EntityTrait>::Model> = query.fetch_page(page).await?;

    let paginated_response = PaginatedResponse {
        rows,
        page: pagination_filter.page,
        page_size: pagination_filter.page_size,
        total_pages: page_count,
    };

    Ok(paginated_response)
}
