use loco::{settings, wizard::RenderingMethodOption};
use rstest::rstest;

use super::*;
use crate::assertion;

pub fn run_generator(asset: RenderingMethodOption) -> TestGenerator {
    let settings = settings::Settings {
        asset: asset.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_middleware_when_asset_empty(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(RenderingMethodOption::None);
    let content = assertion::yaml::load(generator.path(config_file));

    assertion::yaml::assert_path_is_empty(&content, &["server", "middlewares"]);
}

#[rstest]
fn test_config_file_middleware_asset_server(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(RenderingMethodOption::Serverside);
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
fn test_config_file_middleware_asset_client(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(RenderingMethodOption::Clientside);
    let content = assertion::yaml::load(generator.path(config_file));

    assertion::yaml::assert_path_is_object(&content, &["server", "middlewares"]);

    let expected: serde_yaml::Value = serde_yaml::from_str(
        r"
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
",
    )
    .unwrap();
    assertion::yaml::assert_path_value_eq(&content, &["server", "middlewares"], &expected);
}

#[rstest]
fn test_cargo_toml(
    #[values(RenderingMethodOption::None, RenderingMethodOption::Serverside, RenderingMethodOption::Clientside)]
    asset: RenderingMethodOption,
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
    #[values(RenderingMethodOption::None, RenderingMethodOption::Serverside, RenderingMethodOption::Clientside)]
    asset: RenderingMethodOption,
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
        RenderingMethodOption::Serverside | RenderingMethodOption::None => {
            assertion::string::assert_not_contains(&content, frontend_section);
        }
        RenderingMethodOption::Clientside => {
            assertion::string::assert_contains(&content, frontend_section);
        }
    }
}
