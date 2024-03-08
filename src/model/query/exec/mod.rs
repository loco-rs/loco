use sea_orm::{prelude::*, Condition, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    model::query::{PaginatedInfoResponse, PaginatedResponse, PaginationQuery},
    Result as LocoResult,
};

/// Paginate function for fetching paginated data from the database.
///
/// # Examples
///
/// Without conditions
/// ```
/// use loco_rs::tests_cfg::db::*;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = dummy_connection().await;
///     let pagination_query = model::query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     
///     let res = model::query::exec::paginate(&db, test_db::Entity::find(), None, &pagination_query).await;
/// }
/// ````
/// With conditions
/// ```
/// use loco_rs::tests_cfg::db::*;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = dummy_connection().await;
///     let pagination_query = model::query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     let condition = model::query::dsl::condition().contains(test_db::Column::Name, "loco").build();
///     let res = model::query::exec::paginate(&db, test_db::Entity::find(), Some(condition), &pagination_query).await;
/// }
/// ````
/// With Order By
/// ```
/// use loco_rs::tests_cfg::db::*;
/// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait, sea_query::Order, QueryOrder};
/// use loco_rs::prelude::*;
///
/// async fn example() {
///     let db = dummy_connection().await;
///     let pagination_query = model::query::PaginationQuery {
///         page_size: 100,
///         page: 1,
///     };
///     
///     let condition = model::query::dsl::condition().contains(test_db::Column::Name, "loco").build();
///     let entity = test_db::Entity::find().order_by(test_db::Column::Name, Order::Desc);
///     let res = model::query::exec::paginate(&db, entity, Some(condition), &pagination_query).await;
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
    let entity = if let Some(condition) = condition {
        entity.filter(condition)
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
