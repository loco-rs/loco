#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::module_name_repetitions)]
#![doc = include_str!("../README.md")]

pub use self::errors::Error;

mod banner;
pub mod prelude;

#[cfg(feature = "with-db")]
pub mod doctor;

#[cfg(feature = "with-db")]
pub mod db;
#[cfg(feature = "with-db")]
pub mod model;
#[cfg(feature = "with-db")]
pub mod schema;
mod tera;

pub mod app;
#[cfg(feature = "cli")]
pub mod cli;

pub mod auth;
pub mod boot;
pub mod cache;
pub mod config;
pub mod controller;
pub mod environment;
pub mod errors;
mod gen;
pub mod hash;
mod logger;
pub mod mailer;
mod redis;
pub mod task;
#[cfg(feature = "testing")]
pub mod testing;
#[cfg(feature = "testing")]
pub use axum_test::TestServer;
pub mod storage;
pub mod validation;
pub mod worker;
#[cfg(feature = "channels")]
pub use socketioxide;
#[cfg(feature = "testing")]
pub mod tests_cfg;
pub use validator;

/// Application results options list
pub type Result<T, E = Error> = std::result::Result<T, E>;
