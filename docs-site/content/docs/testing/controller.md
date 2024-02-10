+++
title = "Controller"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 22
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

When testing controllers, the goal is to call the router's controller endpoint and verify the HTTP response, including the status code, response content, headers, and more.

To initialize a test request, use `testing::request`, which prepares your app routers, providing the request instance and the application context.

In the following example, we have a POST endpoint that returns the data sent in the POST request.

```rust

#[tokio::test]
#[serial]
async fn can_print_echo() {
    configure_insta!();

    testing::request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/example")
            .json(&serde_json::json!({"site": "Loco"}))
            .await;

        assert_debug_snapshot!((response.status_code(), response.text()));
    })
    .await;
}
```

As you can see initialize the testing request and using `request` instance calling /example endpoing.
the request returns a `Response` instance with the status code and the response test

## Seeding Data Before Running Tests

To seed data before running tests, refer to the [documentation here](@/docs/testing/overview.md#seeding-data)
