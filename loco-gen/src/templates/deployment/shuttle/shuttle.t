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
  content: |
    shuttle-axum = { version = "{{shuttle_runtime_version}}", default-features = false, features = ["axum-0-7"] }
    shuttle-runtime = { version = "{{shuttle_runtime_version}}", default-features = false }
    {% if with_db -%}
    shuttle-shared-db = { version = "{{shuttle_runtime_version}}", features = ["postgres"] }
    {%- endif %}
---
use loco_rs::boot::{create_app, StartMode};
use loco_rs::environment::Environment;
use {{pkg_name}}::app::App;
{% if with_db %}use migration::Migrator;{% endif %}
use shuttle_runtime::DeploymentMetadata;

#[shuttle_runtime::main]
async fn main(
  {% if with_db %}#[shuttle_shared_db::Postgres] conn_str: String,{% endif %}
  #[shuttle_runtime::Metadata] meta: DeploymentMetadata,
) -> shuttle_axum::ShuttleAxum {
    {% if with_db %}std::env::set_var("DATABASE_URL", conn_str);{% endif %}
    let environment = match meta.env {
        shuttle_runtime::Environment::Local => Environment::Development,
        shuttle_runtime::Environment::Deployment => Environment::Production,
    };
    let boot_result = create_app::<App{% if with_db %}, Migrator{% endif %}>(StartMode::ServerOnly, &environment)
        .await
        .unwrap();

    let router = boot_result.router.unwrap();
    Ok(router.into())
}
