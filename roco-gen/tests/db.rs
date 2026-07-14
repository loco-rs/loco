use duct::cmd;
use insta::assert_snapshot;
use roco_gen::get_mappings;
use rstest::rstest;
use serial_test::serial;
use std::{collections::HashMap, env::current_dir, fs::read_to_string};

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
    let roco_dev_path = current_dir().unwrap();
    let roco_dev_path = roco_dev_path.parent().unwrap();
    // 1. install most recent dev cli: cd roco-new; cargo install --path . --force
    // 2. when running locally set ROCO_DEV_MODE_PATH=<to local roco path>
    // ROCO_DEV_MODE_PATH=../../ cargo run -- new

    let mut env_map: HashMap<String, String> = std::env::vars().collect();
    env_map.insert(
        "ROCO_DEV_MODE_PATH".into(),
        roco_dev_path.to_str().unwrap().to_string(),
    );

    if db_kind == "sqlite" {
        env_map.remove("DATABASE_URL");
    }

    cmd!(
        "roco",
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
        "roco db reset",
        "roco g model user name:string",
        "roco g model user_without_tz name:string --without-tz",
        &format!("roco g scaffold playlists {types_line} --htmx"),
        &format!("roco g scaffold playlists_without_tz {types_line} --htmx --without-tz"),
        &format!("roco g model movies {types_line} playlist:references user:references?"),
        "roco g migration AddContentToMovies content:string",
        "roco g migration CreateActors foobar:string",
        // TBD this errors under sqlite because they don`t support alter and uniq
        //        &format!("roco g migration AddAllToActors {types_line}"),
        "roco g migration CreateJoinTableActorsAndMovies minutes:int",
        "roco g migration CreateJoinTableUser_without_tzAndMovies minutes:int --without-tz",
        "roco g migration CreateAwards name:string actor:references",
        "roco g migration RemoveContentFromMovies content:string",
        "roco g migration AddRatingToMovies rating:int",
        "roco db migrate",
        "roco db entities",
        "roco db schema",
    ];

    for line in script {
        cmd("cargo", line.split(' '))
            .full_env(&env_map)
            .dir(tree_fs.root.join("myapp"))
            .run()
            .unwrap_or_else(|_| panic!("command {line} should run successfully"));
    }
    // cargo roco build
    assert_snapshot!(
        format!("migrations_flow_{db_kind}"),
        read_to_string(tree_fs.root.join("myapp").join("schema_dump.json")).unwrap()
    );
}
