use duct::cmd;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;

const FOLDER_EXAMPLES: &str = "examples";
const FOLDER_STARTERS: &str = "starters";
const FOLDER_LOCO_CLI: &str = "loco-cli";

const FMT_TEST: [&str; 3] = ["test", "--all-features", "--all"];
const FMT_ARGS: [&str; 4] = ["fmt", "--all", "--", "--check"];
const FMT_CLIPPY: [&str; 8] = [
    "clippy",
    "--",
    "-W",
    "clippy::pedantic",
    "-W",
    "rust-2021-compatibility",
    "-W",
    "rust-2018-idioms",
];

#[derive(Default, Debug)]
pub struct RunResults {
    pub path: PathBuf,
    pub fmt: bool,
    pub clippy: bool,
    pub test: bool,
}

/// Run CI on all Loco resources (lib, cli, starters, examples, etc.).
pub fn all_resources(base_dir: &Path) -> Vec<RunResults> {
    let mut result = vec![];
    result.push(run(base_dir).expect("loco lib mast be tested"));
    result.extend(inner_folders(base_dir, FOLDER_EXAMPLES));
    result.extend(inner_folders(base_dir, FOLDER_STARTERS));
    result.extend(inner_folders(base_dir, FOLDER_LOCO_CLI));

    result
}

/// Run CI on inner folders.
///
/// For example, run CI on all examples/starters folders dynamically by selecting the first root folder and running CI one level down.
pub fn inner_folders(base_dir: &Path, folder: &str) -> Vec<RunResults> {
    let paths = fs::read_dir(base_dir.join(folder)).unwrap();
    let mut results = vec![];

    for path in paths {
        if let Some(res) = run(&path.unwrap().path()) {
            results.push(res);
        }
    }
    results
}

/// Run the entire CI flow on the given folder path.
///
/// Returns `None` if it is not a Rust folder.
pub fn run(dir: &Path) -> Option<RunResults> {
    if dir.join("Cargo.toml").exists() {
        Some(RunResults {
            path: dir.to_path_buf(),
            fmt: cargo_fmt(dir).is_ok(),
            clippy: cargo_clippy(dir).is_ok(),
            test: cargo_test(dir).is_ok(),
        })
    } else {
        None
    }
}

/// Run cargo test on the given directory.
fn cargo_test(dir: &Path) -> Result<Output, std::io::Error> {
    println!(
        "Running `cargo {}` in folder {}",
        FMT_TEST.join(" "),
        dir.display()
    );
    cmd("cargo", FMT_TEST.as_slice()).dir(dir).run()
}

/// Run cargo fmt on the given directory.
fn cargo_fmt(dir: &Path) -> Result<Output, std::io::Error> {
    println!(
        "Running `cargo {}` in folder {}",
        FMT_ARGS.join(" "),
        dir.display()
    );
    cmd("cargo", FMT_ARGS.as_slice()).dir(dir).run()
}

/// Run cargo clippy on the given directory.
fn cargo_clippy(dir: &Path) -> Result<Output, std::io::Error> {
    println!(
        "Running `cargo {}` in folder {}",
        FMT_CLIPPY.join(" "),
        dir.display()
    );
    cmd("cargo", FMT_CLIPPY.as_slice()).dir(dir).run()
}
