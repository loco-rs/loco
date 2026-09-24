use loco_rs::prelude::*;

/// Serves the server-side rendered home page at `/`.
///
/// Without this the server-side starter had nothing at the root: it shipped
/// `assets/views/home/hello.html` and a test that rendered it, but no route
/// pointed at the template, so choosing `--assets serverside` and running the
/// app gave a 404 on the first page anyone visits.
///
/// It is also the only place a generated app demonstrates
/// `format::render().view(..)`. The controller takes the `ViewRenderer` trait
/// through the `ViewEngine` extractor rather than touching Tera directly, so
/// swapping the engine later leaves this call site compiling untouched.
#[debug_handler]
async fn index(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/hello.html", data!({}))
}

pub fn routes() -> Routes {
    Routes::new().add("/", get(index))
}
