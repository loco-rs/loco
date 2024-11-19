use std::sync::OnceLock;

use axum::{http, routing::MethodRouter};
use regex::Regex;

use crate::app::AppContext;

static DESCRIBE_METHOD_ACTION: OnceLock<Regex> = OnceLock::new();

fn get_describe_method_action() -> &'static Regex {
    DESCRIBE_METHOD_ACTION.get_or_init(|| Regex::new(r"\b(\w+):\s*BoxedHandler\b").unwrap())
}

/// Extract the allow list method actions from [`MethodRouter`].
///
/// Currently axum not exposed the action type of the router. for hold extra
/// information about routers we need to convert the `method` to string and
/// capture the details
pub fn method_action(method: &MethodRouter<AppContext>) -> Vec<http::Method> {
    let method_str = format!("{method:?}");

    get_describe_method_action()
        .captures(&method_str)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_lowercase()))
        .and_then(|method_name| match method_name.as_str() {
            "get" => Some(http::Method::GET),
            "post" => Some(http::Method::POST),
            "put" => Some(http::Method::PUT),
            "delete" => Some(http::Method::DELETE),
            "head" => Some(http::Method::HEAD),
            "options" => Some(http::Method::OPTIONS),
            "connect" => Some(http::Method::CONNECT),
            "patch" => Some(http::Method::PATCH),
            "trace" => Some(http::Method::TRACE),
            _ => {
                tracing::info!("Unknown method: {}", method_name);
                None
            }
        })
        .into_iter()
        .collect::<Vec<_>>()
}
