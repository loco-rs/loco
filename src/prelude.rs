pub use async_trait::async_trait;
pub use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
pub use axum_extra::extract::cookie;
pub use chrono::NaiveDateTime as DateTime;
pub use include_dir::{include_dir, Dir};
#[cfg(feature = "with-db")]
pub use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, ModelTrait, Set};

#[cfg(any(feature = "auth_jwt", feature = "with-db"))]
pub use crate::controller::middleware::auth;
pub use crate::{
    app::{AppContext, Initializer},
    controller::{
        format,
        middleware::format::{Format, RespondTo},
        not_found, unauthorized,
        views::{engines::TeraView, ViewEngine, ViewRenderer},
        Json, Routes,
    },
    errors::Error,
    mailer,
    mailer::Mailer,
    task::{Task, TaskInfo},
    worker::{self, AppWorker},
    Result,
};

#[cfg(feature = "with-db")]
pub mod model {
    pub use crate::model::query;
}
