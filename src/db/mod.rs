//! # Database Operations
//!
//! This module defines functions and operations related to the application's
//! database interactions.
//!
//! ## Supported backends
//!
//! Loco officially supports **`PostgreSQL`** and **`SQLite`**. Schema
//! introspection, seeding, and autoincrement/sequence handling are implemented
//! for those two backends only; any other backend returns
//! [`sea_orm::DbErr::BackendNotSupported`] (with the operation name as its
//! context). Core migration and connection paths may work against additional
//! backends via Sea-ORM, but they are neither guaranteed nor tested here.

mod connect;
mod entities;
mod introspect;
mod migrate;
mod seed;

pub use connect::*;
pub use entities::*;
pub use introspect::*;
pub use migrate::*;
pub use seed::*;

pub(crate) const IGNORED_TABLES: &[&str] = &[
    "seaql_migrations",
    "pg_loco_queue",
    "sqlt_loco_queue",
    "sqlt_loco_queue_lock",
];
