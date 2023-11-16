//! This module contains utility functions for generating HTTP responses that
//! are commonly used in web applications. These functions simplify the process
//! of creating responses with various data types.
//!
//! # Example:
//!
//! This example illustrates how to construct a JSON-formatted response using a
//! Rust struct.
//!
//! ```rust
//! use rustyrails::{controller::format, Result};
//! use axum::Json;
//!
//! pub struct Health {
//!     pub ok: bool,
//! }
//!
//! async fn ping() -> Result<Json<Health>> {
//!    format::json(Health { ok: true })
//! }
//! ```

use axum::Json;

use crate::Result;

/// Returns an empty response.
///
/// # Example:
///
/// This example illustrates how to return an empty response.
/// ```rust
/// use rustyrails::{controller::format, Result};
///
/// async fn endpoint() -> Result<()> {
///    format::empty()
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn empty() -> Result<()> {
    Ok(())
}

/// Returns a response containing the provided text.
///
/// # Example:
///
/// This example illustrates how to return an text response.
/// ```rust
/// use rustyrails::{controller::format, Result};
///
/// async fn endpoint() -> Result<String> {
///    format::text("MESSAGE-RESPONSE")
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn text(t: &str) -> Result<String> {
    Ok(t.to_string())
}

/// Returns a JSON response containing the provided data.
///
/// # Example:
///
/// This example illustrates how to construct a JSON-formatted response using a
/// Rust struct.
///
/// ```rust
/// use rustyrails::{
///     controller::format,
///     Result,
/// };
/// use axum::Json;
///
/// pub struct Health {
///     pub ok: bool,
/// }
///
/// async fn endpoint() -> Result<Json<Health>> {
///    format::json(Health { ok: true })
/// }
/// ```
///
/// # Errors
///
/// Currently this function did't return any error. this is for feature
/// functionality
pub fn json<T>(t: T) -> Result<Json<T>> {
    Ok(Json(t))
}
