use loco_rs::{
    app::{AppContext, Initializer},
    axum::{async_trait, Extension, Router as AxumRouter},
    controller::views::{ViewEngine, ViewRenderer},
    Result,
};
use serde::Serialize;

#[derive(Clone)]
pub struct HelloView;
impl ViewRenderer for HelloView {
    fn render<S: Serialize>(&self, _key: &str, _data: S) -> Result<String> {
        Ok("hello".to_string())
    }
}

pub struct HelloViewEngineInitializer;
#[async_trait]
impl Initializer for HelloViewEngineInitializer {
    fn name(&self) -> String {
        "custom-view-engine".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        Ok(router.layer(Extension(ViewEngine::from(HelloView))))
    }
}
