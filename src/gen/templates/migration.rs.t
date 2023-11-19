to: controllers/<%= name %>.rs
injections:
- in: controllers/mod.rs
  content: "mod <%= plural(name) %>;"
  skip_if: <some regex> 
  replace: <expr, captures>/\1,val\2
  before: <some regex> (work on lines)
  after: <some regex> (work on lines)
  append: true 
  prepend: true
- in: app.rs
  content: "mod <%= plural(name) %>;"
  skip_if: <some regex> 
  replace: <expr, captures>/\1,val\2
  before: <some regex> (work on lines)
  after: <some regex> (work on lines)
  append: true 
  prepend: true
run_after: cargo fmt controllers/mod.rs app.rs controllers/<%= name %>.rs
---
/*
1. gen file: controllers/users.rs
2. inject controllers/mod.rs
3. inject app.rs
*/
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
