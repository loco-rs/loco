+++
title = "Channels"
description = ""
date = 2024-01-21T18:20:00+00:00
updated = 2024-01-21T18:20:00+00:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++


`Loco` support opening channels sockets by enable the `channels` feature flag. Socket implementation is based on `socketioxide` [crate](https://crates.io/crates/socketioxide).


## How To Configure Channels
Once the `channels` feature is enabled, you need to implement the `register_channels` hook in the `Hooks` of the `app.rs` file. The `register_channels` function should return an `AppChannels` instance, describing all the channels to be registered into Loco.



### Creating Channel Code

Begin by creating a folder called `channels`. In this folder, create an `application.rs` file, ensuring it is included in the `mod.rs` file. Inside `application.rs`, define the following function:

```rust
use loco_rs::socketioxide::{
    extract::{AckSender, Bin, Data, SocketRef},
    SocketIo,
};

use serde_json::Value;
fn on_connect(socket: SocketRef, Data(data): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);
    socket.emit("auth", data).ok();

    socket.on(
        "message",
        |socket: SocketRef, Data::<Value>(data), Bin(bin)| {
            info!("Received event: {:?} {:?}", data, bin);
            socket.bin(bin).emit("message-back", data).ok();
        },
    );

    socket.on(
        "message-with-ack",
        |Data::<Value>(data), ack: AckSender, Bin(bin)| {
            info!("Received event: {:?} {:?}", data, bin);
            ack.bin(bin).send(data).ok();
        },
    );

``` 

Next, register the `on_connect` function in Loco routes. Navigate to `app.rs` and add `register_channels` to the `Hooks` app. After registering the channel, go to the routes hook and add the `AppChannels` instance to `AppRouter`.

```rust
use crate::channels;
pub struct App;
#[async_trait]
impl Hooks for App {
    .
    .
    .
    fn routes(ctx: &AppContext) -> AppRoutes {
        AppRoutes::empty()
        .prefix("/api")
        .add_app_channels(Self::register_channels(ctx))
    }

    fn register_channels(_ctx: &AppContext) -> AppChannels {
        let channels = AppChannels::default();
        channels.register.ns("/", channels::application::on_connect);
        channels

    }
    .
    .
    .
}
```
For a simple example of a chat room implementation, refer to this [link](https://github.com/loco-rs/chat-rooms).
