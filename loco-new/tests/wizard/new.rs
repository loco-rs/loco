use std::{fs, path::PathBuf, sync::Arc};

use duct::cmd;
use loco::{
    generator::{executer::FileSystem, Generator},
    settings, wizard,
    wizard_opts::{AssetsOption, BackgroundOption, DBOption},
};
use rstest::rstest;
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

#[rstest]
fn new_from_wizard(
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
    use std::collections::HashMap;

    let test_dir = TestDir::new();

    let executor = FileSystem::new(&PathBuf::from("base_template"), &test_dir.path);

    let wizard_selection = wizard::Selections {
        db,
        background,
        asset,
    };
    let settings = settings::Settings::from_wizard("test-loco-template", &wizard_selection);

    let res = Generator::new(Arc::new(executor), settings).run();
    assert!(res.is_ok());

    let mut env_map: HashMap<_, _> = std::env::vars().collect();
    env_map.insert("RUSTFLAGS".into(), "-D warnings".into());
    assert!(cmd!("cargo", "check")
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
