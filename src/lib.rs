#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::module_name_repetitions)]
//! `TODO::`
//!
//!
//! ## Starting A New Project
//!
//! To start a new project, you can use cargo-generate:
//!
//! ```
//! cargo install cargo-generate
//! cargo generate https://github.com/rustyrails-rs/rustyrails-demo-template
//! ```
//!
//! ## Available Features
//!
//! To avoid compiling unused dependencies, rustyrails gates certain features.
//!
//! | Feature   | Default | Description                 |
//! |-----------|---------|-----------------------------|
//! | `auth`    | true    | Enable user authentication. |
//! | `cli`     | true    | Expose Cli commands.        |
use self::errors::Error;

pub mod db;

pub mod app;
#[cfg(feature = "cli")]
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
