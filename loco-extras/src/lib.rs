//! # Loco Extras
//!
//! Loco Extras provides a collection of common implementations that prove to be generally useful when working with [Loco](https://loco.rs/).
//!
//! ## Features
//!
//! ### Initializers
//! * `initializer-prometheus` For adding prometheus collection metrics
//!   endpoint.
//! * `initializer-extra-db` Adding extra DB connection
//! * `initializer-multi-db` Adding extra DB's connection
//! * `initializer-normalize-path` Normalize the request path
//! * `initializer-opentelemetry` For adding opentelemetry tracing
pub mod initializers;
