//! # Database Operations
//!
//! This module defines functions and operations related to the application's
//! database interactions.

mod connect;
mod entities;
mod migrate;
mod schema;
mod seed;

pub use connect::*;
pub use entities::*;
pub use migrate::*;
pub use schema::*;
pub use seed::*;

pub(crate) const IGNORED_TABLES: &[&str] = &[
    "seaql_migrations",
    "pg_loco_queue",
    "sqlt_loco_queue",
    "sqlt_loco_queue_lock",
];
