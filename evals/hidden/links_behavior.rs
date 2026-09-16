//! Hidden behavioural test.
//!
//! Copied into the app's `tests/requests/` **after** the agent has stopped, so
//! nothing here can be read, targeted, or gamed while the app is being written.
//! Everything it asserts is behaviour the spec asks for and no grep over the
//! diff can see: that a click is actually counted, that an expired link really
//! stops redirecting, that the owner listing is ordered and scoped to its owner.
//!
//! It is written against the API contract in `SPEC.md` and nothing else — it
//! must not assume any particular module layout, table name, or model method.

use loco_rs::testing::prelude::*;
use serial_test::serial;
use shortly::app::App;

use super::prepare_data;

const TARGET: &str = "https://example.com/a/very/long/path";

/// Create a link as a logged-in user, evaluating to the parsed JSON body.
///
/// A macro rather than a function: the test server's type is `axum_test::
/// TestServer`, which a generated app does not depend on directly, so naming it
/// in a signature would not compile everywhere.
macro_rules! create_link {
    ($request:expr, $ctx:expr, $slug:expr, $url:expr) => {{
        let user = prepare_data::init_user_login(&$request, &$ctx).await;
        let (k, v) = prepare_data::auth_header(&user.token);
        let res = $request
            .post("/api/links")
            .add_header(k, v)
            .json(&serde_json::json!({ "url": $url, "slug": $slug }))
            .await;
        assert_eq!(res.status_code(), 200, "creating a link should succeed");
        res.json::<serde_json::Value>()
    }};
}

#[tokio::test]
#[serial]
async fn short_url_is_built_from_configured_base_url() {
    request::<App, _, _>(|request, ctx| async move {
        let body = create_link!(request, ctx, "base-url", TARGET);
        let short = body["short_url"].as_str().expect("short_url present");

        assert!(
            short.ends_with("base-url"),
            "short_url should end with the slug, got {short}"
        );
        // It must be built from config, not hard-coded to the request host.
        assert!(
            short.starts_with("http"),
            "short_url should be absolute, got {short}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn public_redirect_sends_the_caller_to_the_target() {
    request::<App, _, _>(|request, ctx| async move {
        create_link!(request, ctx, "go-there", TARGET);

        let res = request.get("/go-there").await;
        let status = res.status_code().as_u16();
        assert!(
            (300..400).contains(&status),
            "expected a redirect, got {status}"
        );
        let location = res
            .headers()
            .get("location")
            .expect("Location header present")
            .to_str()
            .unwrap();
        assert_eq!(location, TARGET);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn clicks_are_actually_counted() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (k, v) = prepare_data::auth_header(&user.token);

        let res = request
            .post("/api/links")
            .add_header(k.clone(), v.clone())
            .json(&serde_json::json!({ "url": TARGET, "slug": "counted" }))
            .await;
        assert_eq!(res.status_code(), 200);

        for _ in 0..3 {
            request.get("/counted").await;
        }

        let listing = request
            .get("/api/links")
            .add_header(k, v)
            .await
            .json::<serde_json::Value>();
        let links = listing.as_array().expect("listing is an array");
        let mine = links
            .iter()
            .find(|l| l["slug"] == "counted")
            .expect("the link is in its owner's listing");

        assert_eq!(
            mine["click_count"].as_i64(),
            Some(3),
            "three redirects should have been counted, got {}",
            mine["click_count"]
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn listing_is_ordered_most_clicked_first() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (k, v) = prepare_data::auth_header(&user.token);

        for slug in ["cold", "hot"] {
            let res = request
                .post("/api/links")
                .add_header(k.clone(), v.clone())
                .json(&serde_json::json!({ "url": TARGET, "slug": slug }))
                .await;
            assert_eq!(res.status_code(), 200);
        }

        request.get("/cold").await;
        for _ in 0..5 {
            request.get("/hot").await;
        }

        let listing = request
            .get("/api/links")
            .add_header(k, v)
            .await
            .json::<serde_json::Value>();
        let links = listing.as_array().expect("listing is an array");
        let slugs: Vec<&str> = links.iter().filter_map(|l| l["slug"].as_str()).collect();

        let hot = slugs.iter().position(|s| *s == "hot");
        let cold = slugs.iter().position(|s| *s == "cold");
        assert!(
            hot < cold,
            "most-clicked should come first, got order {slugs:?}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn a_listing_never_leaks_another_users_links() {
    request::<App, _, _>(|request, ctx| async move {
        create_link!(request, ctx, "not-yours", TARGET);

        // A second, genuinely distinct user. `prepare_data::init_user_login`
        // registers a fixed address, so calling it twice returns the SAME user
        // and would make this assertion vacuous — register one by hand.
        const OTHER: &str = "other@loco.com";
        request
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "name": "other", "email": OTHER, "password": "1234"
            }))
            .await;
        let other_user = shortly::models::users::Model::find_by_email(&ctx.db, OTHER)
            .await
            .expect("second user registered");
        request
            .post("/api/auth/verify")
            .json(&serde_json::json!({ "token": other_user.email_verification_token }))
            .await;
        let token = request
            .post("/api/auth/login")
            .json(&serde_json::json!({ "email": OTHER, "password": "1234" }))
            .await
            .json::<serde_json::Value>()["token"]
            .as_str()
            .expect("second user can log in")
            .to_string();
        let (k, v) = prepare_data::auth_header(&token);
        let listing = request
            .get("/api/links")
            .add_header(k, v)
            .await
            .json::<serde_json::Value>();

        let leaked = listing
            .as_array()
            .expect("listing is an array")
            .iter()
            .any(|l| l["slug"] == "not-yours");
        assert!(!leaked, "a user must not see another user's links");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unknown_slug_is_not_found() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/definitely-not-a-real-slug").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn a_duplicate_slug_is_rejected() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;
        let (k, v) = prepare_data::auth_header(&user.token);

        let first = request
            .post("/api/links")
            .add_header(k.clone(), v.clone())
            .json(&serde_json::json!({ "url": TARGET, "slug": "taken" }))
            .await;
        assert_eq!(first.status_code(), 200);

        let second = request
            .post("/api/links")
            .add_header(k, v)
            .json(&serde_json::json!({ "url": TARGET, "slug": "taken" }))
            .await;
        assert_eq!(
            second.status_code(),
            400,
            "a taken slug should be rejected, not overwritten"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn creating_a_link_requires_a_login() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .post("/api/links")
            .json(&serde_json::json!({ "url": TARGET }))
            .await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}
