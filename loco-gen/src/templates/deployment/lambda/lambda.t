to: "src/bin/lambda.rs"
skip_exists: true
message: "Lambda entrypoint + cargo-lambda config generated. Deploy in two commands: `cargo lambda build --release --arm64 --output-format zip` then `cargo lambda deploy --enable-function-url` (prints a live HTTPS URL)."
injections:
- into: "Cargo.toml"
  after: "\\[dependencies\\]"
  content: "lambda_http = { version = \"1.3\" }"
- into: "Cargo.toml"
  append: true
  content: |

    # AWS Lambda (cargo-lambda) config.
    # Declarative deploy config, so `cargo lambda build/deploy` need no extra
    # flags. Nothing environment-specific is hardcoded here: region, account,
    # IAM role and secrets are supplied at deploy time (flags / --env-var /
    # Secrets Manager), never committed to source.
    [package.metadata.lambda.build]
    # Files Loco reads from disk at runtime must ship inside the zip. `config/`
    # (config/<env>.yaml) is always required; asset dirs are added automatically
    # when your app serves them. Append any other runtime-read paths.
    include = [{% for p in include %}"{{ p }}"{% if not loop.last %}, {% endif %}{% endfor %}]

    [package.metadata.lambda.deploy]
    memory = 512          # MB — raise for heavier workloads
    timeout = 30          # seconds
    env = { LOCO_ENV = "production" }
---
// AWS Lambda entrypoint for this Loco app.
//
// Loco builds a standard Axum `Router`; both Axum and the Lambda runtime are
// `tower::Service`s, so the same router runs on Lambda with no rewrite. This
// boots the app in `ServerOnly` mode and hands the router to the Lambda HTTP
// runtime. Background workers and the scheduler are intentionally not started
// here — they don't fit Lambda's request/response model; run those on a
// separate always-on target (ECS/EC2) or drive them from SQS/EventBridge.
//
// Deploy (see https://www.cargo-lambda.info) — config lives in Cargo.toml
// [package.metadata.lambda], so these need no extra flags:
//   cargo lambda build --release --arm64 --output-format zip
//   cargo lambda deploy --enable-function-url
// `deploy` creates the function, an execution role, and a public Function URL,
// then prints the HTTPS endpoint. Set secrets with `--env-var KEY=VALUE`.
//
// Keep migrations out of the request path: run `cargo loco db migrate` from CI
// or a one-off task, and set the runtime environment (via `LOCO_ENV`) to a
// config whose `database` does not auto-migrate on boot.
use lambda_http::{run, Error};
use loco_rs::app::Hooks;
use loco_rs::boot::{create_app, StartMode};
use loco_rs::environment::{resolve_from_env, Environment};
{%- if db %}
use migration::Migrator;
{%- endif %}
use {{pkg_name}}::app::App;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let environment: Environment = resolve_from_env().into();
    let config = App::load_config(&environment).await?;
    {%- if db %}
    let boot = create_app::<App, Migrator>(StartMode::ServerOnly, &environment, config).await?;
    {%- else %}
    let boot = create_app::<App>(StartMode::ServerOnly, &environment, config).await?;
    {%- endif %}
    let router = boot
        .router
        .expect("ServerOnly boot always builds a router");
    run(router).await
}
