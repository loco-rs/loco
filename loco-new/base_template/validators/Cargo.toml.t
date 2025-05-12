[package]
name = "validators"
version = "0.1.0"
publish = false
edition.workspace = true

[lib]
name = "validators"
path = "src/lib.rs"

[dependencies]
serde = { workspace = true }

validator = { version = "0.20", features = ["derive"] }
