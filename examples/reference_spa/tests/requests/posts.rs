use loco_rs::{testing::prelude::*, TestServer};
use reference_spa::app::App;
use serial_test::serial;

use super::prepare_data;

/// Creates `count` posts through the API and returns nothing -- the point is
/// the rows, not the responses.
async fn seed_posts(request: &TestServer, token: &str, count: usize) {
    let (auth_key, auth_value) = prepare_data::auth_header(token);
    for i in 0..count {
        let response = request
            .post("/api/posts")
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({
                "title": format!("post {i}"),
                "content": "body",
                "status": "draft",
                "price": "1.5",
                "published_at": null,
            }))
            .await;
        assert_eq!(
            response.status_code(),
            201,
            "seeding post {i} failed: {}",
            response.text()
        );
    }
}

/// The `list` endpoint's response shape is the contract every scaffolded
/// resource emits and the typed SPA consumes (`bindings/Page.ts`). It is a flat
/// envelope whose metadata field names match the framework's own `PagerMeta`,
/// so an app has one pagination vocabulary -- pin all four names, not just the
/// items.
#[tokio::test]
#[serial]
async fn list_answers_with_the_page_envelope() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        seed_posts(&request, &user.token, 3).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/posts")
            .add_header(auth_key, auth_value)
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());

        let body: serde_json::Value = response.json();
        assert_eq!(
            body["items"].as_array().expect("items is an array").len(),
            3
        );
        assert_eq!(body["page"], 1);
        assert_eq!(body["page_size"], 25, "the framework's default page size");
        assert_eq!(body["total_pages"], 1);
        assert_eq!(body["total_items"], 3);
    })
    .await;
}

/// `page`/`page_size` reach the paginator. This is the assertion that the
/// scaffold's old hand-rolled arithmetic and the framework's `query::paginate`
/// have to agree on: page 2 of a 2-per-page list holds the third row, and
/// `total_pages` -- which the scaffold's own envelope did not even carry --
/// reports 2.
#[tokio::test]
#[serial]
async fn list_paginates() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        seed_posts(&request, &user.token, 3).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/posts?page=2&page_size=2")
            .add_header(auth_key, auth_value)
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());

        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().expect("items is an array");
        assert_eq!(items.len(), 1, "page 2 of 3 rows at 2 per page holds one");
        assert_eq!(items[0]["title"], "post 2");
        assert_eq!(body["page"], 2);
        assert_eq!(body["page_size"], 2);
        assert_eq!(body["total_pages"], 2);
        assert_eq!(body["total_items"], 3);
    })
    .await;
}

/// This resource adds a `status` filter alongside the flattened
/// `PaginationQuery`. Both halves of the query string have to survive
/// `#[serde(flatten)]` together -- the filter is useless if supplying it resets
/// pagination to defaults, and vice versa.
#[tokio::test]
#[serial]
async fn list_filters_and_paginates_together() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        seed_posts(&request, &user.token, 3).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/posts?status=draft&page=1&page_size=2")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;

        assert_eq!(response.status_code(), 200, "{}", response.text());

        let body: serde_json::Value = response.json();
        assert_eq!(
            body["items"].as_array().expect("items is an array").len(),
            2
        );
        assert_eq!(body["page_size"], 2, "the filter must not reset pagination");
        assert_eq!(body["total_items"], 3);

        // A filter that matches nothing still answers with a well-formed
        // envelope rather than an empty body.
        let response = request
            .get("/api/posts?status=published")
            .add_header(auth_key, auth_value)
            .await;

        let body: serde_json::Value = response.json();
        assert_eq!(
            body["items"].as_array().expect("items is an array").len(),
            0
        );
        assert_eq!(body["total_items"], 0);
    })
    .await;
}

/// The routes are scaffolded with an `auth::JWT` extractor, so an anonymous
/// caller gets nothing. Worth pinning next to the tests that do send a token:
/// they would all still pass if the extractor were dropped.
#[tokio::test]
#[serial]
async fn list_rejects_an_anonymous_caller() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/posts").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
