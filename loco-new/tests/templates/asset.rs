use loco::{settings, wizard::AssetsOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(asset: AssetsOption) -> TestGenerator {
    let settings = settings::Settings {
        asset: asset.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_middleware_asset_server(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(AssetsOption::Serverside);
    let content = assertion::yaml::load(generator.path(config_file));

    assertion::yaml::assert_path_is_object(&content, &["server", "middlewares", "static"]);

    let expected: serde_yaml::Value = serde_yaml::from_str(
        r"
enable: true
must_exist: true
precompressed: false
folder:
    uri: /static
    path: assets/static
fallback: assets/static/404.html
",
    )
    .unwrap();
    assertion::yaml::assert_path_value_eq(
        &content,
        &["server", "middlewares", "static"],
        &expected,
    );
}
#[rstest]
fn test_config_request_context_session_is_not_empty(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(AssetsOption::Serverside);
    let content = assertion::yaml::load(generator.path(config_file));
    assertion::yaml::assert_path_is_object(&content, &["server", "middlewares", "request_context"]);
    let expected: serde_yaml::Value = serde_yaml::from_str(
        r#"
enable: true
session_config:
  name: "__loco_session"
  http_only: true
  same_site:
    type: Lax
  expiry: 3600
  secure: false
  path: /
# domain: ""
session_store:
type: Cookie
value:
    private_key: ""
"#,
    )
    .unwrap();
    assertion::yaml::assert_path_value_eq_excluded(
        &content,
        &["server", "middlewares", "request_context"],
        &[
            "server",
            "middlewares",
            "request_context",
            "session_store",
            "value",
            "private_key",
        ],
        &expected,
    );
}

#[rstest]
fn test_config_file_middleware_asset_client(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(AssetsOption::Clientside);
    let content = assertion::yaml::load(generator.path(config_file));

    assertion::yaml::assert_path_is_object(&content, &["server", "middlewares"]);

    let expected: serde_yaml::Value = serde_yaml::from_str(
        r#"
fallback:
    enable: false
static:
    enable: true
    must_exist: true
    precompressed: false
    folder:
        uri: /
        path: frontend/dist
    fallback: frontend/dist/index.html
"#,
    )
    .unwrap();
    assertion::yaml::assert_path_value_eq_excluded(
        &content,
        &["server", "middlewares"],
        &["server", "middlewares", "request_context"],
        &expected,
    );
}

#[rstest]
fn test_cargo_toml(
    #[values(AssetsOption::None, AssetsOption::Serverside, AssetsOption::Clientside)]
    asset: AssetsOption,
) {
    let generator = run_generator(asset.clone());
    let content = assertion::toml::load(generator.path("Cargo.toml"));

    insta::assert_snapshot!(
        format!("cargo_dependencies_{:?}", asset),
        content.get("dependencies").unwrap()
    );
}

#[rstest]
fn test_github_ci_yaml(
    #[values(AssetsOption::None, AssetsOption::Serverside, AssetsOption::Clientside)]
    asset: AssetsOption,
) {
    let generator: TestGenerator = run_generator(asset.clone());
    let content =
        assertion::string::load(generator.path(".github").join("workflows").join("ci.yaml"));

    let frontend_section = r"      - name: Setup node
        uses: actions/setup-node@v4
        with:
          node-version: ${{matrix.node-version}}
      - name: Build frontend
        run: npm install && npm run build
        working-directory: ./frontend
      - name: Setup Rust cache
        uses: Swatinem/rust-cache@v2";

    match asset {
        AssetsOption::Serverside | AssetsOption::None => {
            assertion::string::assert_not_contains(&content, frontend_section);
        }
        AssetsOption::Clientside => {
            assertion::string::assert_contains(&content, frontend_section);
        }
    }
}
