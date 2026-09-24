pub use async_trait::async_trait;
pub use axum::{
    debug_handler,
    extract::{Form, Multipart, Path, Query, State},
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
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait,
    DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, ModelTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
// `Expr` is a type, so importing it is inert. `ExprTrait` is deliberately NOT
// here, and the tests at the bottom of this file exist to keep it out.
//
// It is `impl<T> ExprTrait for T where T: Into<Expr>`, and primitives are
// `Into<Expr>`, so exporting it reaches every value in every file that globs
// this prelude. Two distinct failures, both verified by re-adding it:
//
//   - `n.max(1)` / `n.min(1)` stop compiling: E0034 against `Ord`.
//   - `n.eq(&3)` compiles and is *wrong*. `ExprTrait::eq` takes `self` while
//     `PartialEq::eq` takes `&self`, so the by-value candidate wins without
//     ambiguity and you get an `Expr`, not a `bool`.
//
// It was exported briefly to fix `Expr::col(..).like(..)`, and the eval caught
// `params.page.max(1)` failing one run later. Loco already answers that case
// its own way: `query::condition().contains(col, s)` and its `like` /
// `starts_with` / `ends_with` siblings need no trait in scope.
#[cfg(feature = "with-db")]
pub use sea_orm::sea_query::Expr;
// sugar for controller views to use `data!({"item": ..})` instead of `json!`
pub use serde_json::json as data;

#[cfg(feature = "auth")]
pub use crate::controller::extractor::auth;
pub use crate::controller::extractor::{
    shared_store::SharedStore,
    validate::{JsonValidate, JsonValidateWithMessage},
};
#[cfg(feature = "with-db")]
pub use crate::model::{query, Authenticable, ModelError, ModelResult};
#[cfg(feature = "multi-tenancy")]
pub use crate::model::{TenantActiveModelExt, TenantEntity, TenantQueryExt};
pub use crate::{
    app::{AppContext, Initializer},
    bgworker::{BackgroundWorker, Queue},
    controller::{
        bad_request, format,
        middleware::{
            format::{Format, RespondTo},
            remote_ip::RemoteIP,
            MiddlewareStackExt,
        },
        not_found, unauthorized,
        views::{engines::TeraView, ViewEngine, ViewRenderer},
        Json, Routes,
    },
    errors::Error,
    mailer,
    mailer::Mailer,
    task::{self, Task, TaskInfo},
    validation::{self, Validatable, ValidatorTrait},
    Result,
};
// `query::paginate` is in this prelude and returns a `PageResponse` whose
// `meta` is literally a `PagerMeta` — but the pair you render it with was not,
// so the only reachable way to answer a paginated endpoint was to hand-roll an
// envelope carrying the same four fields.
//
// Gated to match the module it re-exports: `views::pagination` is `with-db`,
// and exporting it unconditionally broke every DB-less app at E0432. A gate on
// the definition is not a gate on the re-export.
#[cfg(feature = "with-db")]
pub use crate::controller::views::pagination::{Pager, PagerMeta};
pub use validator::Validate;
#[cfg(feature = "with-db")]
pub mod model {
    pub use crate::model::query;
}
#[cfg(feature = "testing")]
pub use crate::testing::prelude::*;

#[cfg(test)]
mod tests {
    //! A prelude is a glob import in every file of every app, so a trait added
    //! here is added to every value in scope. These call the std methods that a
    //! blanket-impl'd extension trait would shadow; if one stops compiling, the
    //! prelude has started breaking ordinary Rust in user code.
    #![allow(clippy::eq_op)]

    // The glob is the subject of these tests, not a convenience: it is what
    // brings any blanket-impl'd trait into scope. Nothing here names a prelude
    // item on purpose — a test that referenced one would pass even if the
    // shadowing returned.
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn ordinary_std_methods_are_not_shadowed() {
        let page: u64 = 3;
        assert_eq!(page.max(1), 3);
        assert_eq!(page.min(1), 1);
        assert!(page.eq(&3));
        assert!(page.ne(&4));
    }

    #[test]
    fn ordinary_std_ops_are_not_shadowed() {
        use std::ops::{Add, Div, Mul, Sub};
        assert_eq!(6_u64.add(2), 8);
        assert_eq!(6_u64.sub(2), 4);
        assert_eq!(6_u64.mul(2), 12);
        assert_eq!(6_u64.div(2), 3);
    }
}
