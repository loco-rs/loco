use loco_rs::{model::query, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{models::users, views::user_search::UserSummary};

/// Page size when `settings.user_search.page_size` is absent from config.
const DEFAULT_PAGE_SIZE: u64 = 25;

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
    #[serde(default = "default_page")]
    pub page: u64,
}

const fn default_page() -> u64 {
    1
}

/// Search users by name or email.
///
/// Taking `auth::JWT` is what makes this endpoint authenticated: the extractor
/// rejects an unauthenticated request with 401 before this body runs, so there
/// is no "if not logged in" branch to forget.
#[debug_handler]
async fn search(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(params): Query<SearchParams>,
) -> Result<Response> {
    // Page size is configuration, not a literal in the handler.
    let page_size = ctx
        .config
        .settings
        .as_ref()
        .and_then(|s| s.get("user_search"))
        .and_then(|s| s.get("page_size"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(DEFAULT_PAGE_SIZE);

    let pagination = query::PaginationQuery {
        page: params.page,
        page_size,
    };

    let results = users::Model::search(&ctx.db, &params.q, &pagination).await?;

    format::json(UserSummary::page(&results.page, results.meta))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/users").add("/search", get(search))
}
