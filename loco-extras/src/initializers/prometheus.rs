use async_trait::async_trait;
use axum::Router as AxumRouter;
use axum_prometheus::PrometheusMetricLayer;
use loco_rs::prelude::*;

pub struct AxumPrometheusInitializer;

#[async_trait]
impl Initializer for AxumPrometheusInitializer {
    fn name(&self) -> String {
        "axum-prometheus".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
        let router = router
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .layer(prometheus_layer);
        Ok(router)
    }
}
