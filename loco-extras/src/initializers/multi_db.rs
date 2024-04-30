use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use loco_rs::{db, errors::Error, prelude::*};

#[allow(clippy::module_name_repetitions)]
pub struct MultiDbInitializer;

#[async_trait]
impl Initializer for MultiDbInitializer {
    fn name(&self) -> String {
        "multi-db".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let settings = ctx
            .config
            .initializers
            .clone()
            .ok_or_else(|| Error::Message("settings config not configured".to_string()))?;

        let multi_db = settings
            .get("multi_db")
            .ok_or_else(|| Error::Message("multi_db not configured".to_string()))?;

        let multi_db = db::MultiDb::new(serde_json::from_value(multi_db.clone())?).await?;
        Ok(router.layer(Extension(multi_db)))
    }
}
