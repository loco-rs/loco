use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::Output,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

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
    test_combination(db, asset, background, true, false);
}

/// Serverside + `--embedded-assets`: templates are compiled into the binary and
/// served by a different view engine than the on-disk one. That engine has its
/// own constructor path, so the combination needs a real end-to-end build —
/// settings-level assertions alone let a non-compiling app ship.
#[rstest::rstest]
#[serial_test::serial]
fn test_embedded_assets_serverside_builds() {
    test_combination(
        DBOption::Sqlite,
        AssetsOption::Serverside,
        BackgroundOption::Async,
        false,
        true,
    );
}

/// Guarantees one combination at a time, whatever the attributes expand to.
///
/// Every combination builds an app named `test-loco-template` with a
/// `migration v0.1.0` beside it into one shared `CARGO_TARGET_DIR`, so two
/// combinations running at once write the same artifact filenames. `#[serial]`
/// is supposed to prevent that, but it is propagated onto rstest's generated
/// cases by attribute expansion — and it demonstrably did not hold: a matrix
/// run caught app `E9B4V` running `Compiling migration v0.1.0` in the middle of
/// app `MJHmN`'s doctest phase, and that doctest then died on
/// `extern location for migration does not exist` because its rlib had been
/// replaced underneath it. The case passes alone.
///
/// An explicit lock does not care how the attributes expanded. Poisoning is
/// ignored on purpose: a panicking combination has already failed and reported,
/// and turning that into a cascade of poisoned-lock failures in every later
/// combination would bury the one real error.
static ONE_COMBINATION_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn test_combination(
    db: DBOption,
    asset: AssetsOption,
    background: BackgroundOption,
    test_generator: bool,
    embedded_assets: bool,
) {
    let _serialized = ONE_COMBINATION_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let test_dir = tree_fs::TreeBuilder::default().drop(true);

    let executor = FileSystem::new(&PathBuf::from("base_template"), &test_dir.root);

    let wizard_selection = wizard::Selections {
        db: db.clone(),
        background,
        asset: asset.clone(),
    };
    let settings = if embedded_assets {
        settings::Settings::from_wizard_checked(
            "test-loco-template",
            &wizard_selection,
            OS::default(),
            true,
        )
        .expect("serverside + embedded assets is a valid combination")
    } else {
        settings::Settings::from_wizard("test-loco-template", &wizard_selection, OS::default())
    };

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
    //
    // Exception: `embedded_assets` locates the application by walking up from
    // `OUT_DIR` to a directory named `target`, so a relocated target dir makes
    // it embed nothing. That combo builds in-tree instead — see
    // `build/embedded_assets.rs::find_app_directory`.
    if !embedded_assets {
        let shared_target = std::env::temp_dir().join("loco-new-wizard-target");
        env_map.insert(
            "CARGO_TARGET_DIR".into(),
            shared_target.to_string_lossy().into_owned(),
        );
    } else {
        env_map.remove("CARGO_TARGET_DIR");
    }

    let test_dir_root = test_dir.root.clone();
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

    // Compiling is not the same as running. Boot the app and make it answer.
    //
    // Clientside apps are excluded: their static middleware is `must_exist`
    // against `frontend/dist`, which only exists after a frontend build, so a
    // freshly generated one refuses to start by design. That leaves the
    // clientside boot path uncovered here.
    if asset != AssetsOption::Clientside {
        let db_url = format!(
            "sqlite://{}?mode=rwc",
            test_dir_root.join("production.sqlite").display()
        );
        let queue_url = format!(
            "sqlite://{}?mode=rwc",
            test_dir_root.join("production_queue.sqlite").display()
        );

        tester.run_boot("development", &[], &[]);

        // Production is the environment nobody exercises locally, and the one
        // whose config takes no defaults — so this also proves those variables
        // are the ones the config actually asks for.
        tester.run_boot(
            "production",
            &[
                ("DATABASE_URL", &db_url),
                ("QUEUE_URL", &queue_url),
                ("REDIS_URL", "redis://127.0.0.1"),
                ("JWT_SECRET", "test-secret-not-a-real-one"),
                ("HOST", "http://127.0.0.1"),
                ("MAILER_HOST", "localhost"),
                ("MAILER_USER", "test"),
                ("MAILER_PASSWORD", "test"),
            ],
            &[],
        );
    }

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

            // ...and its public counterpart. `--no-auth` removes an argument
            // from five handler signatures; only generating it proves it still
            // compiles, and only requesting it proves the routes are public.
            tester.run_generate(&vec![
                "scaffold",
                "public_notes",
                "title:string",
                "--no-auth",
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

            // Generate a join-table migration. It must join two tables that
            // actually exist: `groups` was never generated, so the join table
            // carried a foreign key to a missing table. SQLite only checks
            // that on write, and nothing here ever deleted a row — until the
            // `user:delete` task did, and every case failed with
            // `no such table: main.groups`.
            tester.run_generate_migration(&vec!["CreateJoinTableUsersAndMovies", "count:int"]);

            // Everything above proves generated code compiles and its tests
            // pass. It does not prove the app *serves* any of it: the boot
            // above ran before the generators, so no generated route existed
            // yet. Boot once more and request them.
            //
            // These two assertions are the documented behaviour of a
            // scaffolded resource, and both had been documented wrongly:
            // authenticated by default (the tutorial's own curl examples
            // returned 401 while claiming 200), and a paginated page envelope
            // (the tutorial showed a bare array).
            if asset != AssetsOption::Clientside {
                tester.run_boot(
                    "development",
                    &[],
                    &[
                        RouteCheck {
                            path: "/api/movies_apis",
                            status: 401,
                            body_contains: &[],
                        },
                        RouteCheck {
                            path: "/api/public_notes",
                            status: 200,
                            body_contains: &[
                                "\"items\"",
                                "\"page\"",
                                "\"page_size\"",
                                "\"total_pages\"",
                                "\"total_items\"",
                            ],
                        },
                    ],
                );
            }
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

/// Ports for the boot tests. Serialized cases, but a killed run can leave a
/// socket in TIME_WAIT, so each boot takes a fresh one.
static NEXT_PORT: AtomicU16 = AtomicU16::new(5350);

/// Kills the spawned server however `run_boot` leaves — including by panic.
///
/// `duct` does not kill a child when its handle drops, and `run_boot`'s own
/// `kill()` sits after every assertion, so it is unreachable once any of them
/// fires: the startup deadline, the `/_health` check, or the harness killing
/// the whole run. The child then outlives the test still **holding its port**,
/// and because `NEXT_PORT` only counts up it is never reclaimed — the next run
/// on that machine binds against a stranger's listener and fails for a reason
/// unrelated to the code under test.
///
/// Not hypothetical: an aborted run left three servers behind, one of them 22
/// minutes old, and the next run's DB-less combos failed on `ConnectionRefused`
/// against a port a dead run still owned.
struct ServerGuard {
    handle: duct::Handle,
}

impl std::ops::Deref for ServerGuard {
    type Target = duct::Handle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        // Both calls are no-ops if the happy path already killed and reaped it.
        //
        // `wait` is not redundant: `kill` only delivers the signal, so without
        // it the child lingers as a zombie holding its PID — which is exactly
        // what the test below caught.
        let _ = self.handle.kill();
        let _ = self.handle.wait();
    }
}

/// The guard's whole point is the path `run_boot` cannot reach, so it is tested
/// directly rather than through a combo: a bare `duct::Handle` survives its own
/// drop, and the guard must not.
///
/// Unix-only: it spawns `sleep` and polls liveness with `ps`, neither of which
/// exists on Windows — there the spawn dies immediately and the pre-drop
/// liveness assertion fails on a child that was never alive.
#[cfg(unix)]
#[test]
fn server_guard_kills_the_child_it_owns() {
    let guard = ServerGuard {
        handle: cmd!("sleep", "300")
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .start()
            .expect("spawn a long-running child"),
    };
    let pids = guard.pids();
    assert_eq!(pids.len(), 1, "one child to track");
    let pid = pids[0];
    assert!(
        process_is_alive(pid),
        "the child is running before the drop"
    );

    drop(guard);

    // `kill` is asynchronous: the signal is delivered, then the kernel reaps.
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_is_alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !process_is_alive(pid),
        "dropping the guard must kill the server; pid {pid} outlived it"
    );
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// One `GET` against a booted app: what the endpoint answered, and what it
/// answered *with*.
///
/// A hand-rolled request keeps the generator's test build free of an HTTP
/// client and TLS stack; "did this return 200 over loopback" needs neither.
/// The body comes back raw — callers only ever substring-match it, so response
/// framing (`content-length` or chunked) does not have to be interpreted.
fn http_get(port: u16, path: &str) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8_lossy(&response).into_owned();

    let status_line = response.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| {
            std::io::Error::other(format!("no status code in response: {status_line:?}"))
        })?;

    let body = response
        .split_once("\r\n\r\n")
        .map_or(String::new(), |(_, body)| body.to_string());

    Ok((status, body))
}

fn http_status(port: u16, path: &str) -> std::io::Result<u16> {
    http_get(port, path).map(|(status, _)| status)
}

/// One assertion about a booted app's HTTP surface: the path to request, the
/// status it must answer, and substrings its body must contain.
struct RouteCheck<'a> {
    path: &'a str,
    status: u16,
    body_contains: &'a [&'a str],
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

    /// Starts the app and proves it serves traffic.
    ///
    /// Everything else here asks whether the generated app *compiles*. Nothing
    /// asked whether it *runs*, which is how an app whose production config was
    /// a 0-byte file, and one pinned to a `loco-rs` that could not parse its own
    /// config, both passed a full green suite. Compiling is not the deliverable;
    /// a process that answers requests is.
    ///
    /// `/_ping` is liveness — the server is accepting connections. `/_health`
    /// additionally reaches the database, queue and cache the config names, so
    /// a 200 there means the config describes infrastructure that exists.
    /// Boots the app and asserts it answers. `routes` adds assertions beyond
    /// the built-in `/_ping`/`/_health` probes — used after the generators run,
    /// so that generated endpoints are not merely compiled but served.
    fn run_boot(&self, environment: &str, extra_env: &[(&str, &str)], routes: &[RouteCheck<'_>]) {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);

        let mut env_map = self.env_map.clone();
        env_map.insert("LOCO_ENV".into(), environment.to_string());
        env_map.insert("PORT".into(), port.to_string());
        env_map.insert("BINDING".into(), "127.0.0.1".into());
        for (key, value) in extra_env {
            env_map.insert((*key).to_string(), (*value).to_string());
        }

        // Build to completion BEFORE the startup clock starts.
        //
        // `cargo loco start` compiles and only then serves, so timing it from
        // spawn measures the build, not the boot. That is not hypothetical: the
        // two serverside combos drew the first ports of a matrix run, paid the
        // coldest compile, and failed on `ConnectionRefused` at 90s while their
        // child was still in rustc — both passed alone against a warm target
        // (67s), and the embedded combo, which builds in-tree with no shared
        // target dir, needs 268s from cold. A deadline that a slow machine can
        // exhaust by compiling is a flake generator, and it reports the boot as
        // broken when the boot was never reached.
        cmd!("cargo", "build")
            .full_env(&env_map)
            .dir(&self.dir)
            .stdout_capture()
            .stderr_capture()
            .run()
            .unwrap_or_else(|e| panic!("could not build the {environment} server: {e}"));

        let server = ServerGuard {
            handle: cmd!("cargo", "loco", "start", "--no-banner")
                .full_env(&env_map)
                .dir(&self.dir)
                .stdout_capture()
                .stderr_capture()
                .unchecked()
                .start()
                .unwrap_or_else(|e| panic!("could not spawn the {environment} server: {e}")),
        };

        // Poll rather than sleep a fixed amount: a cold start is dominated by
        // migrations, which vary. The binary is already built by here, so this
        // budget covers startup alone.
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut last_error;
        let ping = loop {
            if let Some(output) = server
                .try_wait()
                .expect("could not poll the server process")
            {
                panic!(
                    "the {environment} server exited during startup with {}\n--- stdout ---\n{}\n\
                     --- stderr ---\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                );
            }

            match http_status(port, "/_ping") {
                Ok(status) => break status,
                Err(e) => last_error = Some(e),
            }

            if Instant::now() >= deadline {
                // Report what the child was doing, not just that nobody
                // answered. `last_error` is always `ConnectionRefused` here —
                // it says a socket was closed, which is true of a server still
                // starting, one wedged on a migration, and one that never got
                // as far as binding. Only the child's own output separates
                // them, and without it this failure is undiagnosable from a CI
                // log: it took a re-run on a warm machine to learn that the
                // first occurrence was a compile that had not finished.
                let _ = server.kill();
                let logs = server.wait().ok().map_or_else(String::new, |output| {
                    format!(
                        "\n--- stdout ---\n{}\n--- stderr ---\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr),
                    )
                });
                panic!(
                    "the {environment} server never answered on port {port} within 90s: \
                     {last_error:?}{logs}"
                );
            }
            std::thread::sleep(Duration::from_millis(500));
        };

        let health = http_status(port, "/_health");

        let route_results: Vec<_> = routes
            .iter()
            .map(|check| (check, http_get(port, check.path)))
            .collect();

        let _ = server.kill();
        let output = server.wait().ok();

        let logs = output.map_or_else(String::new, |output| {
            format!(
                "\n--- stdout ---\n{}\n--- stderr ---\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )
        });

        assert_eq!(ping, 200, "GET /_ping in {environment}{logs}");
        assert_eq!(
            health.expect("GET /_health"),
            200,
            "GET /_health in {environment}: the app is up but something it is configured to \
             use is not reachable{logs}"
        );

        for (check, result) in route_results {
            let (status, body) =
                result.unwrap_or_else(|e| panic!("GET {} in {environment}: {e}{logs}", check.path));
            assert_eq!(
                status, check.status,
                "GET {} in {environment} answered {status}, body: {body}{logs}",
                check.path
            );
            for needle in check.body_contains {
                assert!(
                    body.contains(needle),
                    "GET {} in {environment} should answer with {needle:?}, body: {body}{logs}",
                    check.path
                );
            }
        }
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
