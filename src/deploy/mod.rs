//! Deployment utilities for Loco applications
//!
//! This module provides deployment functionality for various cloud platforms.
//!
//! # AWS Lambda
//!
//! Deploy your Loco application to AWS Lambda using Cargo Lambda:
//!
//! ```bash
//! cargo loco deploy lambda
//! ```
//!
//! Configure in your `config/development.yaml`:
//!
//! ```yaml
//! lambda:
//!   project_name: my-app
//!   memory_size: 256
//!   timeout: 30
//!   region: us-east-1
//! ```

pub mod lambda;

