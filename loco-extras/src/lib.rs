//! # Loco Extras
//!
//! Loco Extras provides a collection of common implementations that prove to be generally useful when working with [Loco](https://loco.rs/).
//!
//! ## Features
//!
//! ### Initializers
//! * `initializer-prometheus` For adding prometheus collection metrics
//!   endpoint.
//! ### layers
//! * `layer-db` Adding extra DB connection
//! * `layer-multi-db` Adding extra DB's connection
pub mod initializers;
pub mod layers;
