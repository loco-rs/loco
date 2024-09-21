use axum::{async_trait, Router as AxumRouter};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use loco_rs::{
    app::{AppContext, Context, Initializer},
    Error, Result,
};

pub struct OpenTelemetryInitializer;

#[async_trait]
impl Initializer for OpenTelemetryInitializer {
    fn name(&self) -> String {
        "opentelemetry".to_string()
    }

    async fn before_run(&self, _app_context: &dyn Context) -> Result<()> {
        match init_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers() {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to initialize opentelemetry subscriber: {:?}", e);
                Err(Error::Message(e.to_string()))
            }
        }
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &dyn Context) -> Result<AxumRouter> {
        let router = router
            .layer(OtelInResponseLayer::default())
            .layer(OtelAxumLayer::default());
        Ok(router)
    }
}
