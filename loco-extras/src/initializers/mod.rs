//! Initializers
//!
//! Initializers are a way to encapsulate a piece of infrastructure "wiring"
//! that you need to do in your app. read more [here](https://loco.rs/docs/the-app/initializers/)
//!
//! ### How To Use
//!
//! To integrate `loco-extras` into your project, add the following line to your
//! `Cargo.toml` file:
//!
//! ```toml
//! loco-extras = { version = "*",  features = ["FEATURE"] }
//! ```
//!
//! After adding the crate to your project, navigate to the application hooks in
//! `app.rs` and include the `loco_extras` crate:
//!
//! ```rust
//! use loco_extras;
//! ```
//!
//! Once the `loco_extras` crate is included, proceed to the `initializers` hook
//! function and add your initializations. Here's an example:
//!
//! ```rust,ignore
//! impl Hooks for App {
//!     .
//!     .
//!     .
//!     async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
//!        Ok(vec![
//!            Box::new(loco_extras::initializers::[MODULE_NAME]),
//!        ])
//!    }
//! }
//! ```
//!
//! ### Customization
//!
//! The extras initializers are intentionally not designed for extensive
//! modifications. The concept is to use them as-is without complexity. If you
//! need to customize the initializers, copy the relevant file from the
//! `loco_extras` project into your app, adapt it to your requirements, and
//! update the hook to reference the new source.
//!
//! ### Prometheus:
//!```rust
#![doc = include_str!("././prometheus.rs")]
//!````
//! ### Extra Database connection:
//! ```rust
#![doc = include_str!("././extra_db.rs")]
//!````
//! ### Extra Multiple Database Connections:
//! ```rust
#![doc = include_str!("././multi_db.rs")]
//!````
//! ### Normalize path:
//! ```rust
#![doc = include_str!("././normalize_path.rs")]
//!````
#[cfg(feature = "initializer-extra-db")]
pub mod extra_db;
#[cfg(feature = "initializer-mongodb")]
pub mod mongodb;
#[cfg(feature = "initializer-multi-db")]
pub mod multi_db;
#[cfg(feature = "initializer-normalize-path")]
pub mod normalize_path;
#[cfg(feature = "initializer-opentelemetry")]
pub mod opentelemetry;
#[cfg(feature = "initializer-prometheus")]
pub mod prometheus;
