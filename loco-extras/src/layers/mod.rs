//! Layers
//!
//! Here you can get extra Loco layers that going to append into your
//! `[axum::Router]`.
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
//! Once the `loco_extras` crate is included, proceed to the `after_routes` hook
//! function and call the layer that you want to use.
//!
//! ```rust,ignore
//! impl Hooks for App {
//!     .
//!     .
//!     .
//!     async fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
//!        let router =
//!            loco_extras::layers::db::add(router, serde_json::from_value(secondary_db.clone())?)
//!                .await?;
//!
//!        Ok(router)
//!    }
//! }
//! ```
//!
//! ### Customization
//!
//! The extras layers are intentionally not designed for extensive
//! modifications. The concept is to use them as-is without complexity. If you
//! need to customize the layer, copy the relevant file from the
//! `loco_extras` project into your app, adapt it to your requirements, and
//! update the hook to reference the new source.
//!
//! ### Extra Database connection:
//!```rust
#![doc = include_str!("././db.rs")]
//!````
//! ### Extra Multiple Database Connections:
//! ```rust
#![doc = include_str!("././multi_db.rs")]
//!````
#[cfg(feature = "layer-db")]
pub mod db;

#[cfg(feature = "layer-multi-db")]
pub mod multi_db;
