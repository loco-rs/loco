// some types required for controller generators
// sugar for controller views to use `data!({"item": ..})` instead of `json!`
pub use serde_json::json as data;

#[cfg(all(feature = "auth_jwt", feature = "with-db"))]
pub use crate::controller::middleware::auth;
#[cfg(feature = "with-db")]
pub use crate::model::{query, Authenticable, ModelError, ModelResult};
#[cfg(feature = "with-db")]
pub use crate::sea_orm::prelude::{Date, DateTimeWithTimeZone, Decimal, Uuid};
#[cfg(feature = "with-db")]
pub use crate::sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set,
    TransactionTrait,
};
pub use crate::{
    app::{AppContext, Initializer},
    async_trait::async_trait,
    axum::{
        extract::{Form, Path, State},
        response::{IntoResponse, Response},
        routing::{delete, get, head, options, patch, post, put, trace},
    },
    axum_extra::extract::cookie,
    bgworker::{BackgroundWorker, Queue},
    chrono::NaiveDateTime as DateTime,
    controller::{
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
    include_dir::{include_dir, Dir},
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
