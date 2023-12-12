to: "src/bin/shuttle.rs"
skip_exists: true
message: "Shuttle deployment ready do use"
injections:
- into: .cargo/config.toml
  remove_lines: "loco ="
  content: ""
- into: .cargo/config.toml
  after: \[alias\]
  content: "loco = \"run --bin {{pkg_name}}-cli --\""
- into: Cargo.toml
  before: \[dev-dependencies\]
  content: |
    [[bin]]
    name = "{{pkg_name}}"
    path = "src/bin/shuttle.rs"
- into: Cargo.toml
  after: \[dependencies\]
  content: "shuttle-runtime = { version = \"{{shuttle_runtime_version}}\", default-features = false }"
- into: Cargo.toml
  after: \[dependencies\]
  content: "shuttle-axum = { version = \"{{shuttle_axum_version}}\", default-features = false, features = [\"axum-0-7\",] }"
---
use loco_rs::boot::{create_app, StartMode};
use loco_rs::environment::resolve_from_env;
use {{pkg_name}}::app::App;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let environment = resolve_from_env().unwrap_or_else(|| "development".to_string());
    let boot_result = create_app::<App>(StartMode::ServerOnly, &environment)
        .await
        .unwrap();

    let router = boot_result.router.unwrap();
    Ok(router.into())
}
