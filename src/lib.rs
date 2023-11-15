#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::module_name_repetitions)]
//! `TODO::`
//
//
use self::errors::Error;

pub mod db;

pub mod app;
pub mod cli;

#[cfg(feature = "auth")]
pub mod auth;
pub mod boot;
pub mod config;
pub mod controller;
pub mod environment;
pub mod errors;
mod logger;
pub mod mailer;
pub mod model;
mod redis;
pub mod schema;
pub mod task;
pub mod testing;
pub mod validation;
pub mod worker;
pub use validator;

/// Application results options list
pub type Result<T> = std::result::Result<T, Error>;
