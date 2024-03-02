use sea_orm::{prelude::*, Condition, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    model::query::{PaginatedInfoResponse, PaginatedResponse, PaginationQuery},
    Result as LocoResult,
};

/// Paginate function for fetching paginated data from the database.
///
/// # Errors
///
/// Returns a `LocoResult` indicating any errors that occur
/// during pagination.
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
