use std::{collections::HashMap, path::PathBuf, process::Output, sync::Arc};

use duct::cmd;
use loco::{
    generator::{executer::FileSystem, Generator},
    settings,
    wizard::{self, AssetsOption, BackgroundOption, DBOption},
    OS,
};

// #[cfg(feature = "test-wizard")]
// #[rstest::rstest]
// fn test_all_combinations(
//     #[values(DBOption::None, DBOption::Sqlite)] db: DBOption,
//     #[values(
//         BackgroundOption::Async,
//         BackgroundOption::Queue,
//         BackgroundOption::Blocking,
//         BackgroundOption::None
//     )]
//     background: BackgroundOption,
//     #[values(AssetsOption::Serverside, AssetsOption::Clientside,
// AssetsOption::None)]     asset: AssetsOption,
// ) {
//     test_combination(db, background, asset, true);
// }

// when running locally set LOCO_DEV_MODE_PATH=<to local loco path>
#[rstest::rstest]
// Serialized: every combo builds into one shared CARGO_TARGET_DIR (see
// test_combination), so the heavy end-to-end cases must not run concurrently.
#[serial_test::serial]
// lightweight service
#[case(DBOption::None, AssetsOption::None, BackgroundOption::Async)]
// REST API
#[case(DBOption::Sqlite, AssetsOption::None, BackgroundOption::Async)]
// SaaS, serverside
#[case(DBOption::None, AssetsOption::Serverside, BackgroundOption::Async)]
// SaaS, clientside (no db)
#[case(DBOption::None, AssetsOption::Clientside, BackgroundOption::Async)]
// full-stack SPA: db + clientside — the flagship `generate scaffold` path
// (typed backend DTO+controller + React Query hooks/pages + routes injection)
#[case(DBOption::Sqlite, AssetsOption::Clientside, BackgroundOption::Async)]
// full-stack SPA with a SQLite queue backend (-> worker feature)
#[case(
    DBOption::Sqlite,
    AssetsOption::Clientside,
    BackgroundOption::QueueSqlite
)]
fn test_starter_combinations(
    #[case] db: DBOption,
    #[case] asset: AssetsOption,
    #[case] background: BackgroundOption,
) {
    test_combination(db, asset, background, true);
}

fn test_combination(
    db: DBOption,
    asset: AssetsOption,
    background: BackgroundOption,
    test_generator: bool,
) {
    let test_dir = tree_fs::TreeBuilder::default().drop(true);

    let executor = FileSystem::new(&PathBuf::from("base_template"), &test_dir.root);

    let wizard_selection = wizard::Selections {
        db: db.clone(),
        background,
        asset: asset.clone(),
    };
    let settings =
        settings::Settings::from_wizard("test-loco-template", &wizard_selection, OS::default());

    let res = Generator::new(Arc::new(executor), settings.clone()).run();
    assert!(res.is_ok());

    let mut env_map: HashMap<_, _> = std::env::vars().collect();
    env_map.insert("RUSTFLAGS".into(), "-D warnings".into());
    env_map.insert("DB_CONNECT_TIMEOUT".into(), "2000".into());
    env_map.insert("DB_IDLE_TIMEOUT".into(), "2000".into());
    // Build every generated app into ONE shared, persistent target dir instead
    // of a fresh multi-GB `target/` inside each (ephemeral) tree_fs dir. This
    // keeps the dependency build cache warm across combos (loco-rs is compiled
    // once, not per-combo) and — crucially — means an aborted/killed run leaks
    // only small source temp dirs, never gigabytes of build artifacts. The
    // combos run sequentially, so the shared target dir sees no concurrent use.
    let shared_target = std::env::temp_dir().join("loco-new-wizard-target");
    env_map.insert(
        "CARGO_TARGET_DIR".into(),
        shared_target.to_string_lossy().into_owned(),
    );

    let tester = Tester {
        dir: test_dir.root,
        env_map,
    };

    tester
        .run_clippy()
        .expect("run clippy after create new project");

    tester
        .run_test()
        .expect("run test after create new project");

    if test_generator {
        // Generate controller
        tester.run_generate(&vec!["controller", "notes_api", "create_note", "get_note"]);

        // Generate Task
        tester.run_generate(&vec!["task", "list_users"]);

        // Generate Scheduler
        tester.run_generate(&vec!["scheduler"]);

        // Generate Worker (background workers are always enabled)
        tester.run_generate(&vec!["worker", "cleanup"]);

        if settings.mailer {
            // Generate Mailer
            tester.run_generate(&vec!["mailer", "user_mailer"]);
        }

        // Generate deployment nginx
        tester.run_generate(&vec!["deployment", "nginx"]);

        // Generate deployment docker
        tester.run_generate(&vec!["deployment", "docker"]);

        // Generate data
        tester.run_generate(&vec!["data", "stocks"]);

        if db.enable() {
            // Generate Model
            if !settings.auth {
                tester.run_generate(&vec!["model", "users", "name:string", "email:string"]);
            }
            tester.run_generate(&vec!["model", "movies", "title:string", "user:references"]);

            // Generate Scaffold
            tester.run_generate(&vec![
                "scaffold",
                "movies_api",
                "title:string",
                "user:references",
            ]);

            // Generate CreatePosts migration
            tester.run_generate_migration(&vec![
                "CreatePosts",
                "title:string",
                "movies:references",
            ]);

            // Generate AddNameAndAgeToUsers migration
            tester.run_generate_migration(&vec![
                "AddNameAndAgeToUsers",
                "first_name:string",
                "age:int",
            ]);

            // Generate AddNameAndAgeToUsers migration
            tester.run_generate_migration(&vec![
                "RemoveNameAndAgeFromUsers",
                "first_name:string",
                "age:int",
            ]);

            // Generate AddUserRefToPosts migration
            tester.run_generate_migration(&vec!["AddUserRefToPosts", "users:references"]);

            // Generate CreateJoinTableUsersAndGroups migration
            tester.run_generate_migration(&vec!["CreateJoinTableUsersAndGroups", "count:int"]);
        }
    }
}

#[test]
fn embedded_assets_with_clientside_is_rejected() {
    let sel = wizard::Selections {
        db: DBOption::None,
        background: BackgroundOption::Async,
        asset: AssetsOption::Clientside,
    };
    // embedded requested with clientside must be an error
    assert!(settings::Settings::from_wizard_checked(
        "x",
        &sel,
        OS::default(),
        /*embedded=*/ true
    )
    .is_err());
}

#[test]
fn embedded_assets_serverside_enables_feature() {
    let sel = wizard::Selections {
        db: DBOption::None,
        background: BackgroundOption::Async,
        asset: AssetsOption::Serverside,
    };
    let s = settings::Settings::from_wizard_checked("x", &sel, OS::default(), true)
        .expect("serverside+embedded is valid");
    assert!(s.features.names.contains(&"embedded_assets".to_string()));
}

// Regression: a fully flag-driven (non-interactive) `loco new ... --assets
// serverside` must NOT block on the embedded-assets Confirm prompt. With all
// core options supplied, `select_embedded_assets` honors the flag and never
// prompts. (Calling it here in a non-tty test would hang if it prompted.)
#[test]
fn embedded_assets_non_interactive_serverside_does_not_prompt() {
    let base = wizard::ArgsPlaceholder {
        db: Some(DBOption::None),
        bg: Some(BackgroundOption::Async),
        assets: Some(AssetsOption::Serverside),
        embedded_assets: false,
    };
    // no flag -> default false, no prompt
    assert!(
        !wizard::select_embedded_assets(&base, &AssetsOption::Serverside).unwrap(),
        "non-interactive serverside without --embedded-assets should be false"
    );
    // explicit --embedded-assets -> true, still no prompt
    let with_flag = wizard::ArgsPlaceholder {
        embedded_assets: true,
        ..base
    };
    assert!(
        wizard::select_embedded_assets(&with_flag, &AssetsOption::Serverside).unwrap(),
        "--embedded-assets should enable embedding without prompting"
    );
}

struct Tester {
    dir: PathBuf,
    env_map: HashMap<String, String>,
}

impl Tester {
    fn run_clippy(&self) -> Result<Output, std::io::Error> {
        cmd!(
            "cargo",
            "clippy",
            // "--quiet",
            "--",
            "-W",
            "clippy::pedantic",
            "-W",
            "clippy::nursery",
            "-W",
            "rust-2018-idioms",
            "-A",
            "clippy::result_large_err"
        )
        .full_env(&self.env_map)
        // .stdout_null()
        // .stderr_null()
        .dir(&self.dir)
        .run()
    }

    fn run_test(&self) -> Result<Output, std::io::Error> {
        cmd!("cargo", "test")
            // .stdout_null()
            // .stderr_null()
            .full_env(&self.env_map)
            .dir(&self.dir)
            .run()
    }

    fn run_migrate(&self) -> Result<Output, std::io::Error> {
        cmd!("cargo", "loco", "db", "migrate")
            // .stdout_null()
            // .stderr_null()
            .full_env(&self.env_map)
            .dir(&self.dir)
            .run()
    }

    fn run_generate(&self, command: &Vec<&str>) {
        let base_command = vec!["loco", "generate"];

        // Concatenate base_command with the command vector
        let mut args = base_command.clone();
        args.extend(command);

        duct::cmd("cargo", &args)
            // .stdout_null()
            // .stderr_null()
            .full_env(&self.env_map)
            .dir(&self.dir)
            .run()
            .unwrap_or_else(|_| panic!("generate `{}`", command.join(" ")));

        self.run_clippy()
            .unwrap_or_else(|_| panic!("Run clippy after generate `{}`", command.join(" ")));

        self.run_test()
            .unwrap_or_else(|_| panic!("Run Test after generate `{}`", command.join(" ")));
    }

    fn run_generate_migration(&self, command: &Vec<&str>) {
        let base_command = vec!["loco", "generate", "migration"];

        // Concatenate base_command with the command vector
        let mut args = base_command.clone();
        args.extend(command);

        duct::cmd("cargo", &args)
            // .stdout_null()
            // .stderr_null()
            .full_env(&self.env_map)
            .dir(&self.dir)
            .run()
            .unwrap_or_else(|_| panic!("generate `{}`", command.join(" ")));

        self.run_migrate().unwrap_or_else(|_| {
            panic!(
                "Run migrate after creating the migration `{}`",
                command.join(" ")
            )
        });

        self.run_clippy().unwrap_or_else(|_| {
            panic!(
                "Run clippy after generate migration `{}`",
                command.join(" ")
            )
        });

        self.run_test().unwrap_or_else(|_| {
            panic!("Run Test after generate migration `{}`", command.join(" "))
        });
    }
}
