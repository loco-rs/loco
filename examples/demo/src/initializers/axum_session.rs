use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::prelude::*;

pub struct AxumSessionInitializer;

#[async_trait]
impl Initializer for AxumSessionInitializer {
    fn name(&self) -> String {
        "axum-session".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let session_config =
            axum_session::SessionConfig::default().with_table_name("sessions_table");

        let session_store =
            axum_session::SessionStore::<axum_session::SessionNullPool>::new(None, session_config)
                .await
                .unwrap();

        let router = router.layer(axum_session::SessionLayer::new(session_store));

        Ok(router)
    }
}
