---
title: Live page updates (Turbo Streams)
description: Answer form posts with Turbo Stream fragments and push updates to open pages over server-sent events, using loco_rs::controller::turbo.
sidebar:
  order: 19
---

**Goal:** change parts of a page without a full reload, the way Rails does with `turbo_stream.append` / `replace` / `remove` responses and `broadcast_*_to`. Two halves:

- **Responses:** a form post from a Turbo page gets back `<turbo-stream>` fragments instead of a redirect.
- **Push:** other open pages subscribe to a named stream over server-sent events (SSE) and receive the same fragments when something changes.

Everything lives in `loco_rs::controller::turbo`. This assumes a server-rendered app with Tera views. See [Render server-side views](/docs/how-to/render-views).

## 1. Load Turbo in your layout

Loco does not vendor Turbo's JavaScript. Load it from a CDN in your base layout:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/@hotwired/turbo@8.0.12/dist/turbo.es2017-esm.min.js"></script>
```

Turbo now intercepts form submissions and adds `text/vnd.turbo-stream.html` to their `Accept` header.

## 2. Answer a form post with a stream

Take `TurboStreamRequest` to learn whether the request came from Turbo. Answer Turbo requests with a `TurboStream` and everything else with the usual redirect:

```rust
use loco_rs::controller::turbo::{TurboStream, TurboStreamRequest};
use loco_rs::prelude::*;

#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    ViewEngine(v): ViewEngine<TeraView>,
    turbo: TurboStreamRequest,
    Form(params): Form<PostParams>,
) -> Result<Response> {
    let post = posts::Model::create_from(&ctx.db, &params).await?;

    if turbo.0 {
        let stream = TurboStream::prepend("posts", "")
            .render(&v, "posts/_post.html", data!({ "post": post }))?;
        return Ok(stream.into_response());
    }
    format::redirect("/posts")
}
```

The response has the `text/vnd.turbo-stream.html` content type. Its body looks like this:

```html
<turbo-stream action="prepend" target="posts"><template><li id="post_1">…</li></template></turbo-stream>
```

Turbo prepends the template into the element with `id="posts"`.

Each Turbo action has a constructor:

| Constructor | Effect on the target |
|---|---|
| `TurboStream::append(id, html)` | add `html` as the last child |
| `TurboStream::prepend(id, html)` | add `html` as the first child |
| `TurboStream::replace(id, html)` | replace the element itself |
| `TurboStream::update(id, html)` | replace the element's contents |
| `TurboStream::before(id, html)` / `after(id, html)` | insert a sibling |
| `TurboStream::remove(id)` | delete the element |
| `TurboStream::refresh()` | ask the page to reload itself |

`TurboStream::new_all(action, ".css-selector")` targets every element matching a CSS selector. Turbo calls this the `targets` attribute.

The `target` attribute is HTML-escaped. The template is inserted as-is, so it must come from a Tera view (which escapes your data) or from markup you trust.

To change several parts of the page in one response, return `TurboStreams`:

```rust
use loco_rs::controller::turbo::TurboStreams;

Ok(TurboStreams::new()
    .push(TurboStream::remove(format!("post_{id}")))
    .push(TurboStream::update("post_count", count.to_string()))
    .into_response())
```

If you only have a `HeaderMap`, `turbo::accepts_turbo_stream(&headers)` gives the same check as `TurboStreamRequest`.

## 3. Create the broadcaster

A `Broadcaster` keeps one channel per stream name. Insert it into the shared store once, in `Hooks::after_context`:

```rust
use loco_rs::controller::turbo::Broadcaster;

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    ctx.shared_store.insert(Broadcaster::default());
    Ok(ctx)
}
```

`Broadcaster::default()` buffers 64 messages per subscriber. A subscriber that falls further behind skips the messages it missed. Use `Broadcaster::new(capacity)` to change the buffer size.

## 4. Serve the stream and subscribe to it

Add a route that returns the SSE stream:

```rust
use loco_rs::controller::turbo::Broadcaster;

#[debug_handler]
async fn posts_stream(State(ctx): State<AppContext>) -> Result<Response> {
    Ok(Broadcaster::from_store(&ctx.shared_store)?
        .sse("posts")
        .into_response())
}

pub fn routes() -> Routes {
    Routes::new().prefix("streams").add("/posts", get(posts_stream))
}
```

Then add a source element to the page that should receive updates:

```html
<turbo-stream-source src="/streams/posts"></turbo-stream-source>
<ul id="posts">…</ul>
```

The connection stays open, and the stream sends a keep-alive comment every 15 seconds. To follow one record, include it in the stream name, for example `.sse(&format!("post_{id}"))`.

**Access control is up to you.** The SSE handler is a normal handler, so add the same auth extractor you use for the page and check that the user may read the stream before you return it.

## 5. Broadcast changes

Anything holding an `AppContext` can broadcast. This includes handlers, model methods and workers:

```rust
let html = v.render("posts/_post.html", data!({ "post": post }))?;
Broadcaster::from_store(&ctx.shared_store)?
    .broadcast("posts", TurboStream::prepend("posts", html));
```

`broadcast` returns how many subscribers received the message. Zero means nobody is listening. That is not an error, and the stream's channel is dropped until someone subscribes again.

## Limitations

- **Single process only.** The broadcaster lives in memory. A broadcast reaches the browsers connected to the same process, and no others. If you run several app processes behind a load balancer, each one only reaches its own connections.
- **Workers must run in that process.** A broadcast from a worker only reaches browsers when the worker runs inside the server process (`cargo loco start --server-and-worker`, or the `ForegroundBlocking` / `BackgroundAsync` worker modes). A standalone `cargo loco start --worker` process has its own, separate broadcaster.
- **No replay.** A browser that connects after a broadcast never sees that broadcast. Render the current state in the page itself, and use the stream only for changes.

Sending updates between processes (for example through Redis pub/sub) is a planned follow-up and is not built yet.

## Verify

```sh
curl -N localhost:5150/streams/posts
# in another terminal, create a post through the form; the first terminal prints
# data: <turbo-stream action="prepend" target="posts"><template>…</template></turbo-stream>

curl -s -X POST localhost:5150/posts -H 'accept: text/vnd.turbo-stream.html' -d 'title=hi'
```

In a request test, check the content type against `loco_rs::controller::turbo::CONTENT_TYPE`.

## Next

- [Respond with different formats](/docs/how-to/respond-formats)
- [Render server-side views](/docs/how-to/render-views)
- [Add websockets / realtime](/docs/how-to/websockets)
