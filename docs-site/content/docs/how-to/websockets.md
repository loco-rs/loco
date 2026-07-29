+++
title = "Add websockets / realtime"
description = "Loco has no built-in websocket layer — wire up realtime with an external Axum-compatible crate like socketioxide."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 17
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/extras/websocket/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: add realtime, bidirectional communication (chat, live updates, notifications) to a Loco app.

Loco doesn't ship a built-in websocket abstraction. Because a Loco app compiles down to a real `axum::Router<AppContext>` (see [Coming from Axum](@/docs/explanation/coming-from-axum.md)), any Axum-compatible websocket layer mounts onto it the same way it would onto a hand-rolled Axum app — there's no Loco-specific API to learn for this, and no Loco-specific limitation either.

## Chat room example

For a worked example using [socketioxide](https://github.com/Totodore/socketioxide), see the [`loco-rs/chat-rooms`](https://github.com/loco-rs/chat-rooms) reference app. It shows a full chat-room implementation wired into a Loco router.

If you need something other than socketioxide, look for any crate that integrates with `axum::Router` (raw `axum::extract::ws`, `socketioxide`, etc.) and mount it the same way you'd [add a controller](@/docs/how-to/add-controller.md) — as routes on the app's `Router`.
