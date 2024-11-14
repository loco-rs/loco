use std::{fs, path::PathBuf, sync::Arc};

use duct::cmd;
use loco::{
    generator::{executer::FileSystem, Generator},
    settings,
    wizard::{self, AssetsOption, BackgroundOption, DBOption},
    OS,
};
use uuid::Uuid;

struct TestDir {
    pub path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join("loco-test-generator")
            .join(Uuid::new_v4().to_string());

        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(feature = "test-wizard")]
#[rstest::rstest]
fn test_all_combinations(
    #[values(DBOption::None, DBOption::Sqlite)] db: DBOption,
    #[values(
        BackgroundOption::Async,
        BackgroundOption::Queue,
        BackgroundOption::Blocking,
        BackgroundOption::None
    )]
    background: BackgroundOption,
    #[values(AssetsOption::Serverside, AssetsOption::Clientside, AssetsOption::None)]
    asset: AssetsOption,
) {
    test_combination(db, background, asset);
}

// when running locally set LOCO_DEV_MODE_PATH=<to local loco path>
#[test]
fn test_starter_combinations() {
    // lightweight service
    test_combination(DBOption::None, BackgroundOption::None, AssetsOption::None);
    // REST API
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::None,
    );
    // SaaS, serverside
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::Serverside,
    );
    // SaaS, clientside
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::Clientside,
    );
}

fn test_combination(db: DBOption, background: BackgroundOption, asset: AssetsOption) {
    use std::collections::HashMap;

    let test_dir = TestDir::new();

    let executor = FileSystem::new(&PathBuf::from("base_template"), &test_dir.path);

    let wizard_selection = wizard::Selections {
        db,
        background,
        asset,
    };
    let settings =
        settings::Settings::from_wizard("test-loco-template", &wizard_selection, OS::default());

    let res = Generator::new(Arc::new(executor), settings).run();
    assert!(res.is_ok());

    let mut env_map: HashMap<_, _> = std::env::vars().collect();
    env_map.insert("RUSTFLAGS".into(), "-D warnings".into());
    assert!(cmd!(
        "cargo",
        "clippy",
        "--quiet",
        "--",
        "-W",
        "clippy::pedantic",
        "-W",
        "clippy::nursery",
        "-W",
        "rust-2018-idioms"
    )
    .full_env(&env_map)
    // .stdout_null()
    // .stderr_null()
    .dir(test_dir.path.as_path())
    .run()
    .is_ok());

    cmd!("cargo", "test")
        // .stdout_null()
        // .stderr_null()
        .full_env(&env_map)
        .dir(test_dir.path.as_path())
        .run()
        .expect("run test");
}
