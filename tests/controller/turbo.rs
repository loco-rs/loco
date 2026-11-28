use std::time::Duration;

use loco_rs::{
    controller::turbo::{Broadcaster, TurboStream, TurboStreamRequest, CONTENT_TYPE},
    prelude::*,
    tests_cfg,
};

use crate::infra_cfg;

#[tokio::test]
async fn answers_turbo_stream_requests_with_a_stream() {
    let ctx = tests_cfg::app::get_app_context().await;

    #[allow(clippy::items_after_statements)]
    async fn action(turbo: TurboStreamRequest) -> Result<Response> {
        if turbo.0 {
            return Ok(TurboStream::prepend("posts", "<li id=\"post_1\">Hi</li>").into_response());
        }
        format::redirect("/posts")
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", post(action), Some(port)).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let res = client
        .post(get_base_url_port(port))
        .header(
            "accept",
            "text/vnd.turbo-stream.html, text/html, application/xhtml+xml",
        )
        .send()
        .await
        .expect("response");
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], CONTENT_TYPE);
    assert_eq!(
        res.text().await.unwrap(),
        r#"<turbo-stream action="prepend" target="posts"><template><li id="post_1">Hi</li></template></turbo-stream>"#
    );

    let res = client
        .post(get_base_url_port(port))
        .header("accept", "text/html")
        .send()
        .await
        .expect("response");
    assert_eq!(res.status(), 303);

    handle.abort();
}

#[tokio::test]
async fn broadcasts_reach_sse_subscribers() {
    let ctx = tests_cfg::app::get_app_context().await;
    let hub = Broadcaster::default();
    ctx.shared_store.insert(hub.clone());

    #[allow(clippy::items_after_statements)]
    async fn stream(State(ctx): State<AppContext>) -> Result<Response> {
        Ok(Broadcaster::from_store(&ctx.shared_store)?
            .sse("posts")
            .into_response())
    }

    let port = get_available_port().await;
    let handle = infra_cfg::server::start_with_route(ctx, "/", get(stream), Some(port)).await;

    let mut res = reqwest::get(get_base_url_port(port))
        .await
        .expect("response");
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/event-stream");

    for _ in 0..100 {
        if hub.subscriber_count("posts") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(hub.broadcast("posts", TurboStream::remove("post_1")), 1);

    let mut body = String::new();
    while !body.contains("\n\n") {
        let chunk = tokio::time::timeout(Duration::from_secs(5), res.chunk())
            .await
            .expect("event in time")
            .expect("chunk")
            .expect("open stream");
        body.push_str(std::str::from_utf8(&chunk).unwrap());
    }
    assert_eq!(
        body,
        "data: <turbo-stream action=\"remove\" target=\"post_1\"></turbo-stream>\n\n"
    );

    handle.abort();
}

#[tokio::test]
async fn graceful_shutdown_is_not_held_open_by_sse_clients() {
    let ctx = tests_cfg::app::get_app_context().await;
    let hub = Broadcaster::default();
    ctx.shared_store.insert(hub.clone());

    #[allow(clippy::items_after_statements)]
    async fn stream(State(ctx): State<AppContext>) -> Result<Response> {
        Ok(Broadcaster::from_store(&ctx.shared_store)?
            .sse("posts")
            .into_response())
    }

    let router = loco_rs::controller::AppRoutes::empty()
        .add_route(Routes::new().add("/", get(stream)))
        .to_router::<loco_rs::tests_cfg::db::AppHook>(ctx, axum::Router::new())
        .expect("router");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server_hub = hub.clone();
    // The same order as the default `Hooks::serve`: wait for the signal,
    // then end the broadcaster's streams.
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
                server_hub.shutdown();
            })
            .await
    });

    let mut res = reqwest::get(format!("http://{addr}/"))
        .await
        .expect("response");
    assert_eq!(res.status(), 200);
    for _ in 0..100 {
        if hub.subscriber_count("posts") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(hub.subscriber_count("posts"), 1);

    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server stops while an SSE client is connected")
        .expect("server task")
        .expect("serve");
    // The client sees the stream end rather than hang.
    let rest = tokio::time::timeout(Duration::from_secs(5), res.chunk())
        .await
        .expect("stream ends in time");
    assert!(!matches!(rest, Ok(Some(_))));
    assert_eq!(hub.subscriber_count("posts"), 0);
}
