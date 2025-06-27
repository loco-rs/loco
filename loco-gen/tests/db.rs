use std::{collections::HashMap, env::current_dir, fs::read_to_string};

use duct::cmd;
use insta::assert_snapshot;
use loco_gen::get_mappings;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn test_migrations_flow(#[values("postgres", "sqlite")] db_kind: &str) {
    if db_kind == "postgres" && std::env::var("DATABASE_URL").is_err() {
        return;
    }
    let tree_fs = tree_fs::TreeBuilder::default()
        .drop(true)
        .create()
        .expect("Should create temp folder");
    let loco_dev_path = current_dir().unwrap();
    let loco_dev_path = loco_dev_path.parent().unwrap();
    // 1. install most recent dev cli: cd loco-new; cargo install --path . --force
    // 2. when running locally set LOCO_DEV_MODE_PATH=<to local loco path>
    // LOCO_DEV_MODE_PATH=../../ cargo run -- new

    let mut env_map: HashMap<String, String> = std::env::vars().collect();
    env_map.insert(
        "LOCO_DEV_MODE_PATH".into(),
        loco_dev_path.to_str().unwrap().to_string(),
    );

    if db_kind == "sqlite" {
        env_map.remove("DATABASE_URL");
    }

    cmd!(
        "loco",
        "new",
        "-n",
        "myapp",
        "--db",
        db_kind,
        "--bg",
        "async",
        "--assets",
        "serverside",
        "-a"
    )
    .full_env(&env_map)
    .dir(&tree_fs.root)
    .run()
    .expect("new");

    // build a mega long all-types "title:string ..." pairs for all types from
    // mappings.json name of column is name of type adjusted with unique, or
    // nonnull, etc arity arguments get manual treatment
    let mappings = get_mappings();
    let mut type_names = mappings
        .all_names()
        .iter()
        // only take non-argument types because its easy
        .filter(|n| mappings.col_type_arity(n).unwrap_or_default() == 0)
        .map(|t| format!("{}:{t}", t.replace('!', "_nonull").replace('^', "_uniq")))
        .collect::<Vec<_>>();

    // push arity arguments manually
    type_names.push("age:decimal_len:8:24".to_string());
    type_names.push("age_nonull:decimal_len!:8:24".to_string());

    if db_kind == "postgres" {
        type_names.push("array_string:array:string".to_string());
        type_names.push("array_float:array:float".to_string());
        type_names.push("array_int:array:int".to_string());
        type_names.push("array_double:array:double".to_string());
        type_names.push("array_bool:array:bool".to_string());
    }

    let types_line = type_names.join(" ");

    let script = [
        "loco db reset",
        &format!("loco g scaffold playlists {types_line} --htmx"),
        &format!("loco g model movies {types_line} playlist:references"),
        "loco g migration AddContentToMovies content:string",
        "loco g migration CreateActors foobar:string",
        // TBD this errors under sqlite because they don`t support alter and uniq
        //        &format!("loco g migration AddAllToActors {types_line}"),
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
            .dir(tree_fs.root.join("myapp"))
            .run()
            .unwrap_or_else(|_| panic!("command {line} should run successfully"));
    }
    // cargo loco build
    assert_snapshot!(
        format!("migrations_flow_{db_kind}"),
        read_to_string(tree_fs.root.join("myapp").join("schema_dump.json")).unwrap()
    );
}
