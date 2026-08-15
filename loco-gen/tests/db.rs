use duct::cmd;
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;
use std::{collections::HashMap, env::current_dir, fs::read_to_string};

mod postgres;

#[rstest]
#[serial]
fn test_migrations_flow(#[values("postgres", "sqlite")] db_kind: &str) {
    // Postgres is the half most likely to diverge — arrays, `alter table`,
    // unique indexes — and for a long time it was the half that never ran:
    // the case returned early when `DATABASE_URL` was unset and still reported
    // `ok`, so a local "2 passed" meant SQLite twice.
    //
    // An explicit `DATABASE_URL` still wins, so CI or a local Postgres stays
    // the fast path. Otherwise a container is started, and if that fails the
    // test fails — there is no configuration in which this silently covers
    // nothing.
    let postgres = (db_kind == "postgres").then(postgres::url_or_container);

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
    if let Some(postgres) = &postgres {
        // Explicit rather than `set_var`: the generated app reads the
        // environment this map becomes, and mutating the process environment
        // from a test is both unsafe in edition 2024 and invisible here.
        env_map.insert("DATABASE_URL".into(), postgres.url.clone());
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
    // `column::scalar_from_base_name`'s recognized base names (the successor to
    // the now-removed `mappings.json`); name of column is name of type
    // adjusted with unique, or nonnull, etc. arity arguments get manual
    // treatment. `bool`/`tstz` have no `^` (unique) variant -- see
    // `column::parse_column`'s rejection of `bool^`/`tstz^`.
    let base_names = [
        "string",
        "text",
        "uuid",
        "bool",
        "date",
        "time",
        "date_time",
        "tstz",
        "json",
        "jsonb",
        "blob",
        "money",
        "decimal",
        "float",
        "double",
        "small_int",
        "small_unsigned",
        "unsigned",
        "int",
        "big_int",
    ];
    let mut type_names = Vec::new();
    for base in base_names {
        type_names.push(format!("{base}:{base}"));
        type_names.push(format!("{base}_nonull:{base}!"));
        // bool/tstz have no unique column type; `json` has no btree operator
        // class on Postgres (use `jsonb`). See `column::parse_column`'s rejects.
        if base != "bool" && base != "tstz" && base != "json" {
            type_names.push(format!("{base}_uniq:{base}^"));
        }
    }

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
        "loco g model user name:string",
        "loco g model user_without_tz name:string --without-tz",
        &format!("loco g scaffold playlists {types_line}"),
        &format!("loco g scaffold playlists_without_tz {types_line} --without-tz"),
        &format!("loco g model movies {types_line} playlist:references user:references?"),
        "loco g migration AddContentToMovies content:string",
        "loco g migration CreateActors foobar:string",
        // TBD this errors under sqlite because they don`t support alter and uniq
        //        &format!("loco g migration AddAllToActors {types_line}"),
        "loco g migration CreateJoinTableActorsAndMovies minutes:int",
        "loco g migration CreateJoinTableUser_without_tzAndMovies minutes:int --without-tz",
        "loco g migration CreateAwards name:string actor:references",
        "loco g migration RemoveContentFromMovies content:string",
        "loco g migration AddRatingToMovies rating:int",
        // No fields: the new name is carried by the migration name itself, and
        // the column keeps the type it already had.
        "loco g migration RenameRatingToScoreOnMovies",
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
