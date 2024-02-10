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
#[cfg(feature = "initializer-prometheus")]
pub mod prometheus;
