use loco::{
    settings,
    wizard::{AssetsOption, DBOption},
};
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

/// `src/dtos` only ships for a db app, so anything asserting on it needs a
/// database as well as an asset choice.
pub fn run_generator_with_db(asset: AssetsOption) -> TestGenerator {
    let settings = settings::Settings {
        asset: asset.into(),
        db: DBOption::Sqlite.into(),
        ..Default::default()
    };

    TestGenerator::generate(settings)
}

#[rstest]
fn test_config_file_middleware_when_asset_empty(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(AssetsOption::None);
    let content = assertion::yaml::load(generator.path(config_file));

    assertion::yaml::assert_path_is_empty(&content, &["server", "middlewares"]);
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
fn test_config_file_middleware_asset_client(
    #[values("config/development.yaml", "config/test.yaml")] config_file: &str,
) {
    let generator: TestGenerator = run_generator(AssetsOption::Clientside);
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

/// The asset tree itself, not the config that points at it.
///
/// Everything above asserts the YAML each choice produces; nothing asserted the
/// files. A serverside app whose `assets/` never landed serves a `static`
/// middleware pointed at an empty directory — green config, 404s at runtime.
#[test]
fn serverside_ships_the_assets_the_config_points_at() {
    let generator = run_generator(AssetsOption::Serverside);

    for file in [
        "assets/static/404.html",
        "assets/views/home/hello.html",
        "assets/i18n/en-US/main.ftl",
        "assets/i18n/uk-UA/main.ftl",
        "assets/shared.ftl",
    ] {
        assert!(
            generator.path(file).exists(),
            "{file} is missing, but the serverside config points at it"
        );
    }
}

#[rstest]
fn no_asset_tree_without_a_serverside_app(
    #[values(AssetsOption::None, AssetsOption::Clientside)] asset: AssetsOption,
) {
    let generator = run_generator(asset);

    assert!(
        !generator.path("assets/views/home/hello.html").exists(),
        "server-rendered views were generated for a non-serverside app"
    );
}

/// The shipped Tera view has to be reachable over HTTP.
///
/// `assets/views/home/hello.html` shipped for a year with a test that rendered
/// it directly and no route that served it, so a serverside app 404'd at `/` —
/// the first URL anyone opens. `tests/views` could not catch that: rendering a
/// template and routing to it are different things.
#[test]
fn serverside_routes_a_page_at_the_root() {
    let generator = run_generator(AssetsOption::Serverside);

    let controller = assertion::string::load(generator.path("src/controllers/page.rs"));
    assertion::string::assert_contains(
        &controller,
        r#"format::render().view(&v, "home/hello.html""#,
    );
    assertion::string::assert_contains(&controller, r#"Routes::new().add("/", get(index))"#);

    let modules = assertion::string::load(generator.path("src/controllers/mod.rs"));
    assertion::string::assert_contains(&modules, "pub mod page;");

    let app = assertion::string::load(generator.path("src/app.rs"));
    assertion::string::assert_contains(&app, "controllers::page::routes()");
}

#[rstest]
fn no_root_page_controller_without_a_serverside_app(
    #[values(AssetsOption::None, AssetsOption::Clientside)] asset: AssetsOption,
) {
    let generator = run_generator(asset);

    assert!(
        !generator.path("src/controllers/page.rs").exists(),
        "the server-rendered page controller was generated for a non-serverside app"
    );

    let modules = assertion::string::load(generator.path("src/controllers/mod.rs"));
    assert!(
        !modules.contains("pub mod page;"),
        "controllers/mod.rs declares `page` in a non-serverside app"
    );
}

/// `#[ts(export)]` writes TypeScript during `cargo test`, so it must only
/// appear when there is a frontend to write into.
///
/// `src/dtos` ships for every db app — the scaffolded API controller imports
/// `Page` and `ApiError` regardless of asset choice — but the unconditional
/// export attribute meant running the test suite in an `--assets none` app
/// conjured a `frontend/src/bindings/` tree out of nothing.
#[rstest]
fn dtos_only_export_typescript_when_there_is_a_frontend(
    #[values(AssetsOption::None, AssetsOption::Serverside)] asset: AssetsOption,
) {
    let generator = run_generator_with_db(asset);
    let dtos = assertion::string::load(generator.path("src/dtos/common.rs"));

    assert!(
        !dtos.contains("ts(export"),
        "an app with no frontend still writes ts-rs bindings on `cargo test`:\n{dtos}"
    );
}

#[test]
fn clientside_dtos_export_typescript_for_the_spa() {
    let generator = run_generator_with_db(AssetsOption::Clientside);
    let dtos = assertion::string::load(generator.path("src/dtos/common.rs"));

    assertion::string::assert_contains(
        &dtos,
        r#"#[ts(export, export_to = "../frontend/src/bindings/")]"#,
    );
}
