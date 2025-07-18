pub use async_trait::async_trait;
pub use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, head, options, patch, post, put, trace},
};
pub use axum_extra::extract::cookie;
pub use chrono::NaiveDateTime as DateTime;
pub use include_dir::{Dir, include_dir};
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
pub use crate::controller::extractor::auth;
#[cfg(feature = "with-db")]
pub use crate::model::{Authenticable, ModelError, ModelResult, query};
pub use crate::{
    Result,
    app::{AppContext, Initializer},
    bgworker::{BackgroundWorker, Queue},
    controller::{
        Json, Routes, bad_request,
        extractor::{
            shared_store::SharedStore,
            validate::{JsonValidate, JsonValidateWithMessage},
        },
        format,
        middleware::{
            format::{Format, RespondTo},
            remote_ip::RemoteIP,
        },
        not_found, unauthorized,
        views::{ViewEngine, ViewRenderer, engines::TeraView},
    },
    errors::Error,
    mailer,
    mailer::Mailer,
    task::{self, Task, TaskInfo},
    validation::{self, Validatable},
    validator::Validate,
};
#[cfg(feature = "with-db")]
pub mod model {
    pub use crate::model::query;
}
#[cfg(feature = "testing")]
pub use crate::testing::prelude::*;
