use std::sync::Arc;

use crate::controllers;
use async_trait::async_trait;
use axum::Extension;
use kalosm::language::{Llama, LlamaSource};
use loco_rs::{
    app::{AppContext, Hooks},
    boot::{create_app, BootResult, StartMode},
    controller::AppRoutes,
    task::Tasks,
    worker::Processor,
    Result,
};
use migration::Migrator;
use tokio::sync::RwLock;

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    async fn boot(mode: StartMode, environment: &str) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment).await
    }

    async fn before_run(_ctx: &AppContext) -> Result<()> {
        // force static load now
        Ok(())
    }

    fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
        // cache should reside at: ~/.cache/huggingface/hub
        println!("loading model");
        let model = Llama::builder()
            .with_source(LlamaSource::llama_7b_code())
            .build()
            .unwrap();
        println!("model ready");
        let st = Arc::new(RwLock::new(model));
        Ok(router.layer(Extension(st)))
    }
    fn routes() -> AppRoutes {
        AppRoutes::with_default_routes().add_route(controllers::home::routes())
    }

    fn connect_workers<'a>(_p: &'a mut Processor, _ctx: &'a AppContext) {}

    fn register_tasks(tasks: &mut Tasks) {}
}
