use duct::cmd;
use loco::{
    generator::{executer::FileSystem, Generator},
    settings,
    wizard::{self, AssetsOption, BackgroundOption, DBOption},
    OS,
};
use std::{collections::HashMap, path::PathBuf, process::Output, sync::Arc};

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
    test_combination(db, background, asset, false);
}

// when running locally set LOCO_DEV_MODE_PATH=<to local loco path>
#[test]
fn test_starter_combinations() {
    // lightweight service
    test_combination(
        DBOption::None,
        BackgroundOption::None,
        AssetsOption::None,
        true,
    );
    // REST API
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::None,
        true,
    );
    // SaaS, serverside
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::Serverside,
        true,
    );
    // SaaS, clientside
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::Async,
        AssetsOption::Clientside,
        true,
    );
    // test only DB
    test_combination(
        DBOption::Sqlite,
        BackgroundOption::None,
        AssetsOption::None,
        true,
    );
}

fn test_combination(
    db: DBOption,
    background: BackgroundOption,
    asset: AssetsOption,
    test_generator: bool,
) {
    let test_dir = tree_fs::TreeBuilder::default().drop(true);

    let executor = FileSystem::new(&PathBuf::from("base_template"), &test_dir.root);

    let wizard_selection = wizard::Selections {
        db: db.clone(),
        background: background.clone(),
        asset,
    };
    let settings =
        settings::Settings::from_wizard("test-loco-template", &wizard_selection, OS::default());

    let res = Generator::new(Arc::new(executor), settings.clone()).run();
    assert!(res.is_ok());

    let mut env_map: HashMap<_, _> = std::env::vars().collect();
    env_map.insert("RUSTFLAGS".into(), "-D warnings".into());

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
        // Generate API controller
        tester.run_generate(&vec![
            "controller",
            "notes_api",
            "--api",
            "create_note",
            "get_note",
        ]);

        // Generate HTMX controller
        tester.run_generate(&vec![
            "controller",
            "notes_htmx",
            "--htmx",
            "create_note",
            "get_note",
        ]);

        // Generate HTML controller
        tester.run_generate(&vec![
            "controller",
            "notes_html",
            "--html",
            "create_note",
            "get_note",
        ]);

        // Generate Task
        tester.run_generate(&vec!["task", "list_users"]);

        // Generate Scheduler
        tester.run_generate(&vec!["scheduler"]);

        if background.enable() {
            // Generate Worker
            tester.run_generate(&vec!["worker", "cleanup"]);
        }

        if settings.mailer {
            // Generate Mailer
            tester.run_generate(&vec!["mailer", "user_mailer"]);
        }

        // Generate deployment nginx
        tester.run_generate(&vec!["deployment", "--kind", "nginx"]);

        // Generate deployment nginx
        tester.run_generate(&vec!["deployment", "--kind", "docker"]);

        // Generate deployment shuttle
        tester.run_generate(&vec!["deployment", "--kind", "shuttle"]);

        if db.enable() {
            // Generate Model
            if !settings.auth {
                tester.run_generate(&vec!["model", "users", "name:string", "email:string"]);
            }
            tester.run_generate(&vec!["model", "movies", "title:string", "user:references"]);

            // Generate HTMX Scaffold
            tester.run_generate(&vec![
                "scaffold",
                "movies_htmx",
                "title:string",
                "user:references",
                "--htmx",
            ]);

            // Generate HTML Scaffold
            tester.run_generate(&vec![
                "scaffold",
                "movies_html",
                "title:string",
                "user:references",
                "--html",
            ]);

            // Generate API Scaffold
            tester.run_generate(&vec![
                "scaffold",
                "movies_api",
                "title:string",
                "user:references",
                "--api",
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

struct Tester {
    dir: PathBuf,
    env_map: HashMap<String, String>,
}

impl Tester {
    fn run_clippy(&self) -> Result<Output, std::io::Error> {
        cmd!(
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
