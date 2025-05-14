{%- set_global feature_list = [] -%}
{%- if settings.features.names | length > 0 -%}
    {%- for name in settings.features.names -%}
        {%- set_global feature_list = feature_list | concat(with=['"' ~ name ~ '"']) -%}
    {%- endfor -%}
{%- endif -%}
[workspace]

[package]
name = "{{settings.package_name}}"
version = "0.1.0"
edition = "2021"
publish = false
default-run = "{{settings.module_name}}-cli"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[workspace.dependencies]
loco-rs = { {{settings.loco_version_text}} {%- if not settings.features.default_features  %}, default-features = false {%- endif %} }

[dependencies]
loco-rs = { workspace = true {% if feature_list | length > 0 %}, features = {{feature_list}}{% endif %} }
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1" }
tokio = { version = "1.45", default-features = false, features = [
  "rt-multi-thread",
] }
async-trait = { version = "0.1" }
axum = { version = "0.8" }
tracing = { version = "0.1" }
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
regex = { version = "1.11" }
{%- if settings.db %}
migration = { path = "migration" }
sea-orm = { version = "1.1", features = [
  "sqlx-sqlite",
  "sqlx-postgres",
  "runtime-tokio-rustls",
  "macros",
] }
chrono = { version = "0.4" }
validator = { version = "0.20" }
uuid = { version = "1.6", features = ["v4"] }
{%- endif %}

{%- if settings.mailer %}
include_dir = { version = "0.7" }
{%- endif %}

{%- if settings.asset %}
{%- if settings.asset.kind == "server" %}
# view engine i18n
fluent-templates = { version = "0.13", features = ["tera"] }
unic-langid = { version = "0.9" }
# /view engine
{%- endif %}
axum-extra = { version = "0.10", features = ["form"] }
{%- endif %}

[[bin]]
name = "{{settings.module_name}}-cli"
path = "src/bin/main.rs"
required-features = []

{%- if settings.os == "windows" %}
[[bin]]
name = "tool"
path = "src/bin/tool.rs"
required-features = []
{%- endif %}

[dev-dependencies]
loco-rs = { workspace = true, features = ["testing"] }
serial_test = { version = "3.1.1" }
rstest = { version = "0.25" }
insta = { version = "1.34", features = ["redactions", "yaml", "filters"] }
