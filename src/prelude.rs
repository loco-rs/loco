pub use async_trait::async_trait;
pub use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, head, options, patch, post, put, trace},
};
pub use axum_extra::extract::cookie;
pub use chrono::NaiveDateTime as DateTime;
pub use include_dir::{include_dir, Dir};
// some types required for controller generators
#[cfg(feature = "with-db")]
pub use sea_orm::prelude::{Date, DateTimeUtc, DateTimeWithTimeZone, Decimal, Uuid};
#[cfg(feature = "with-db")]
pub use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set,
    TransactionTrait,
};
// sugar for controller views to use `data!({"item": ..})` instead of `json!`
pub use serde_json::json as data;

#[cfg(all(feature = "auth_jwt", feature = "with-db"))]
pub use crate::controller::middleware::auth;
#[cfg(feature = "with-db")]
pub use crate::model::{query, Authenticable, ModelError, ModelResult};
pub use crate::{
    app::{AppContext, Initializer},
    bgworker::{BackgroundWorker, Queue},
    controller::{
        bad_request,
        extractor::validate::{JsonValidate, JsonValidateWithMessage},
        format,
        middleware::{
            format::{Format, RespondTo},
            remote_ip::RemoteIP,
        },
        not_found, unauthorized,
        views::{engines::TeraView, ViewEngine, ViewRenderer},
        Json, Routes,
    },
    errors::Error,
    mailer,
    mailer::Mailer,
    task::{self, Task, TaskInfo},
    validation::{self, Validatable},
    validator::Validate,
    Result,
};
#[cfg(feature = "with-db")]
pub mod model {
    pub use crate::model::query;
}
#[cfg(feature = "testing")]
pub use crate::testing::prelude::*;
