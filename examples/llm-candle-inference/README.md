# `huggingface/candle` LLM Inference example

This example showcases using `candle`, through a higher level library called [kalosm](https://github.com/floneum/floneum/tree/main/interfaces/kalosm).

Looking at inference in Rust, `candle` is probably where we all want to be. Note that `kalosm` GREATLY simplifies text generation with candle, so give it a deep look.

## Points of interest

### This example implements streaming with Axum

We're going to do LLM text generation, the example is configured to work with macOS, using `accelerate`, and exhibits around 2-3 tokens/s on my M1 Mac. Start your app in `release` because we want every inch of performance:

```sh
cargo run --release -- start
```

It may download a large model file, and will take some more time to prepare and load it to memory.

Next, try your first inference request and wait for the tokens to start streaming:

```sh
$ curl -vvv --no-buffer localhost:3000/candle-llm
```

### Adding a global state for your controllers

This is done by using Axum `Extension` state, in the `after_routes` lifecycle hook:

```rust
    async fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
        // cache should reside at: ~/.cache/huggingface/hub
        println!("loading model");
        let model = Llama::builder()
            .with_source(LlamaSource::llama_7b_code())
            .build()
            .unwrap();
        println!("model ready");
        let st = Arc::new(RwLock::new(model));

        Ok(router.layer(Extension(st)))
    }
```

You can add any state with `router.layer(Extension(<..>))`, then consume it in your controller:

```rust
async fn candle_llm(Extension(m): Extension<Arc<RwLock<Llama>>>) -> impl IntoResponse {
    // use `m` from your state extension
    let prompt = "write binary search";
    ...
```

---

Loco is a web and API framework running on Rust.

This is the **Stateless starter** which comes with no database or state dependencies.

## Quick Start

Start your app:

```
$ cargo loco start
Finished dev [unoptimized + debuginfo] target(s) in 21.63s
    Running `target/debug/myapp start`

    :
    :
    :

controller/app_routes.rs:203: [Middleware] Adding log trace id

                      ▄     ▀
                                 ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
 ██████  █████   ███ █████   ███ █████   ███ ▀█
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀

started on port 3000
```

## Getting help

Check out [a quick tour](https://loco.rs/docs/getting-started/tour/) or [the complete guide](https://loco.rs/docs/getting-started/guide/).
