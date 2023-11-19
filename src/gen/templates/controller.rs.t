to: controllers/<%= name %>.rs
injections:
- in: controllers/mod.rs
  skip_if: <%= plural(name) %>
  content: "mod <%= plural(name) %>;"
  append: true 
- in: app.rs
  content: "foobar..."
run_after: cargo fmt controllers/mod.rs app.rs controllers/<%= name %>.rs
---
use rustyrails::{
    app::AppContext,
    controller::{format, Routes},
    errors::Error,
    worker::AppWorker,
    Result,
};

pub async fn index(State(ctx): State<AppContext>) -> Result<String> {
    format::text("hello")
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(hello))
}
