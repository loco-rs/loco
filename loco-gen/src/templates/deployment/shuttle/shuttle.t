to: "src/bin/shuttle.rs"
skip_exists: true
message: "Shuttle deployment ready do use"
injections:
- into: Cargo.toml
  after: \[dependencies\]
  content: |
    shuttle-axum = "{{shuttle_runtime_version}}"
    shuttle-runtime = { version = "{{shuttle_runtime_version}}", default-features = false }
    {% if with_db -%}
    shuttle-shared-db = { version = "{{shuttle_runtime_version}}", features = ["postgres"] }
    {%- endif %}
---
use loco_rs::boot::{create_app, StartMode};
use loco_rs::environment::Environment;
use {{pkg_name}}::app::App;
{% if with_db %}use migration::Migrator;{% endif %}

#[shuttle_runtime::main]
async fn main(
    {% if with_db %}#[shuttle_shared_db::Postgres] conn_str: String,{% endif %}
    #[shuttle_runtime::Metadata] meta: shuttle_runtime::DeploymentMetadata,
) -> shuttle_axum::ShuttleAxum {
    {% if with_db %}std::env::set_var("DATABASE_URL", conn_str);{% endif %}
    let environment = match meta.env {
        shuttle_runtime::Environment::Local => Environment::Development,
        shuttle_runtime::Environment::Deployment => Environment::Production,
    };

    let config = environment
        .load()
        .expect("Failed to load configuration from the environment");

    let boot_result = create_app::<App{% if with_db %}, Migrator{% endif %}>(StartMode::ServerOnly, &environment, config)
        .await
        .unwrap();

    let router = boot_result.router.unwrap();
    Ok(router.into())
}
