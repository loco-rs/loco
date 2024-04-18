use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{FromRef, FromRequestParts, Request},
    response::Response,
};
use futures_util::{future::BoxFuture, FutureExt};
use loco_rs::prelude::{auth::JWTWithUser, *};
use tower::{BoxError, Layer, Service};

use crate::models::{roles, sea_orm_active_enums::RolesName, users};
