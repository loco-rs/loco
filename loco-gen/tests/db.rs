use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, read_to_string},
    path::PathBuf,
};

use duct::cmd;
use insta::assert_snapshot;
use serial_test::serial;
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

#[test]
#[serial]
fn test_migrations_flow() {
    let test_dir = TestDir::new();
    let loco_dev_path = current_dir().unwrap();
    let loco_dev_path = loco_dev_path.parent().unwrap();
    // when running locally set LOCO_DEV_MODE_PATH=<to local loco path>
    // LOCO_DEV_MODE_PATH=../../ cargo run -- new

    let mut env_map: HashMap<_, _> = std::env::vars().collect();
    env_map.insert(
        "LOCO_DEV_MODE_PATH".into(),
        loco_dev_path.to_str().unwrap().to_string(),
    );
    cmd!(
        "loco",
        "new",
        "-n",
        "myapp",
        "--db",
        "sqlite",
        "--bg",
        "async",
        "--assets",
        "serverside",
        "-a"
    )
    .full_env(&env_map)
    .dir(test_dir.path.as_path())
    .run()
    .expect("new");

    let script = [
        "loco g scaffold playlists title:string --htmx",
        "loco g model movies title:string playlist:references",
        "loco g migration AddContentToMovies content:string",
        "loco g migration CreateActors name:string experience:int age:decimal_len:8:24",
        "loco g migration CreateJoinTableActorsAndMovies minutes:int",
        "loco g migration CreateAwards name:string actor:references",
        "loco g migration RemoveContentFromMovies content:string",
        "loco g migration AddRatingToMovies rating:int",
        "loco db migrate",
        "loco db entities",
        "loco db schema",
    ];

    for line in script {
        cmd("cargo", line.split(' '))
            .full_env(&env_map)
            .dir(test_dir.path.join("myapp"))
            .run()
            .expect("scaffold");
    }
    assert_snapshot!(read_to_string(test_dir.path.join("myapp").join("schema_dump.json")).unwrap());
}
