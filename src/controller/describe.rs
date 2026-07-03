use std::sync::OnceLock;

use axum::{http, routing::MethodRouter};
use regex::Regex;

use crate::app::AppContext;

static DESCRIBE_METHOD_ACTION: OnceLock<Regex> = OnceLock::new();

fn get_describe_method_action() -> &'static Regex {
    DESCRIBE_METHOD_ACTION.get_or_init(|| Regex::new(r"\b(\w+):\s*(BoxedHandler|Route)\b").unwrap())
}

/// Extract the allow list method actions from [`MethodRouter`].
///
/// Currently axum not exposed the action type of the router. for hold extra
/// information about routers we need to convert the `method` to string and
/// capture the details
pub fn method_action(method: &MethodRouter<AppContext>) -> Vec<http::Method> {
    let method_str = format!("{method:?}");

    get_describe_method_action()
        .captures_iter(&method_str)
        .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_lowercase()))
        .filter_map(|method_name| match method_name.as_str() {
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
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use axum::routing::get;

    use super::*;
    use crate::prelude::*;

    async fn handler() -> Result<Response> {
        format::json(())
    }

    /// Sanity check: this is the real `Debug` shape axum 0.8's `MethodRouter`
    /// produces for a route registered with multiple verbs, captured directly
    /// from `format!("{:?}", get(handler).post(handler))`. Only fields whose
    /// value is `BoxedHandler`/`Route` correspond to a verb actually
    /// registered on the route (unregistered verbs print `None`).
    #[test]
    fn multi_verb_debug_format_matches_expected_shape() {
        let method: MethodRouter<AppContext> = get(handler).post(handler);
        let debug_str = format!("{method:?}");
        assert!(debug_str.contains("get: BoxedHandler"));
        assert!(debug_str.contains("post: BoxedHandler"));
        assert!(debug_str.contains("put: None"));
    }

    #[test]
    fn extracts_all_verbs_for_multi_method_route() {
        let method: MethodRouter<AppContext> = get(handler).post(handler);
        assert_eq!(
            method_action(&method),
            vec![http::Method::GET, http::Method::POST]
        );
    }

    #[test]
    fn extracts_single_verb_for_single_method_route() {
        let method: MethodRouter<AppContext> = get(handler);
        assert_eq!(method_action(&method), vec![http::Method::GET]);
    }
}
