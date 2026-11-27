use chrono::{Duration, Utc};
use loco_rs::{prelude::*, tests_cfg};
use serde_json::json;

use crate::infra_cfg;

const PAGE: &str = "{{ post.created_at | time_ago_in_words }} ago
{{ comments | pluralize(word=\"comment\") }}
{{ authors | to_sentence }}
{{ rank | ordinalize }}
{{ views | number_with_delimiter }}
{{ views | number_to_human }}
{{ views | number_to_human(precision=0) }}
{{ started | distance_of_time_in_words(to=finished) }}";

async fn page() -> Result<Response> {
    format::template(
        PAGE,
        json!({
            "post": {"created_at": Utc::now() - Duration::days(3)},
            "comments": 2,
            "authors": ["Ana", "Bo", "Cy"],
            "rank": 22,
            "views": 1_234_567,
            "started": "2024-06-01T12:00:00Z",
            "finished": "2024-06-01T14:10:00Z",
        }),
    )
}

/// The view helpers are registered as Tera filters and reach templates a
/// handler renders.
#[tokio::test]
async fn view_helper_filters_render_in_a_response() {
    let ctx = tests_cfg::app::get_app_context().await;

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(page), Some(port)).await;

    let res = reqwest::get(get_base_url_port(port))
        .await
        .expect("valid response");
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    assert_eq!(
        res.text().await.unwrap(),
        "3 days ago\n2 comments\nAna, Bo, and Cy\n22nd\n1,234,567\n1.23 Million\n1 Million\nabout 2 hours"
    );

    handle.abort();
}
