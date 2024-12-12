use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};

use crate::{
    app::{AppContext, Initializer},
    db, Error, Result,
};

#[allow(clippy::module_name_repetitions)]
pub struct ExtraDbInitializer;

#[async_trait]
impl Initializer for ExtraDbInitializer {
    fn name(&self) -> String {
        "extra_db".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let extra_db_config = ctx
            .config
            .initializers
            .clone()
            .ok_or_else(|| Error::Message("initializers config not configured".to_string()))?;

        let extra_db_value = extra_db_config
            .get("extra_db")
            .ok_or_else(|| Error::Message("initializers config not configured".to_string()))?;

        let extra_db = serde_json::from_value(extra_db_value.clone())?;

        let db = db::connect(&extra_db).await?;
        Ok(router.layer(Extension(db)))
    }
}
