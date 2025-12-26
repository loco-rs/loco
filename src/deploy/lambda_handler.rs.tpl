//! AWS Lambda handler for Loco application (auto-generated)
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use loco_rs::boot::{create_app, StartMode};
use loco_rs::environment::Environment;
use {{APP_MODULE}}::app::App;
{{MIGRATION_IMPORT}}
use std::sync::Arc;
use tokio::sync::OnceCell;

static APP_ROUTER: OnceCell<Arc<axum::Router>> = OnceCell::const_new();

async fn get_router() -> &'static Arc<axum::Router> {
    APP_ROUTER
        .get_or_init(|| async {
            let env_str = std::env::var("LOCO_ENV").unwrap_or_else(|_| "development".to_string());
            let environment: Environment = env_str.parse().unwrap_or(Environment::Development);
            
            let config = environment
                .load()
                .expect(&format!(
                    "Failed to load configuration for environment '{}'. \
                    Make sure config/{}.yaml exists and has all required fields (logger, server, etc.)",
                    env_str, env_str
                ));

            let boot_result = create_app::<App{{MIGRATOR_GENERIC}}>(
                StartMode::ServerOnly,
                &environment,
                config,
            )
            .await
            .expect("Failed to create app");

            Arc::new(boot_result.router.expect("Router not available"))
        })
        .await
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let router = get_router().await;
    let (parts, body) = event.into_parts();
    let body = axum::body::Body::from(body.to_vec());
    let request = axum::http::Request::from_parts(parts, body);
    
    let response = tower::ServiceExt::oneshot(router.clone().as_ref().clone(), request)
        .await
        .map_err(|e| Error::from(format!("Request failed: {}", e)))?;
    
    let (parts, body) = response.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| Error::from(format!("Body conversion failed: {}", e)))?;
    
    Ok(Response::from_parts(parts, Body::from(body_bytes.to_vec())))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}

