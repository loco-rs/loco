use std::{
    path::{Path, PathBuf},
    process::Output,
};

use duct::cmd;

use crate::{errors::Result, utils};

const FMT_TEST: [&str; 3] = ["test", "--all-features", "--all"];
const FMT_ARGS: [&str; 2] = ["fmt", "--all"];
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

impl RunResults {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.fmt && self.clippy && self.test
    }
}

/// Run CI on all Loco resources (lib, cli, starters, examples, etc.).
///
/// # Errors
/// when could not run ci on the given resource
pub fn all_resources(base_dir: &Path) -> Result<Vec<RunResults>> {
    let mut result = vec![];
    result.push(run(base_dir).expect("loco lib mast be tested"));
    result.extend(run_all_in_folder(&base_dir.join("examples"))?);
    result.extend(run_all_in_folder(&base_dir.join("loco-new"))?);

    Ok(result)
}

/// Run CI on inner folders.
///
/// For example, run CI on all examples/starters folders dynamically by
/// selecting the first root folder and running CI one level down.
///
/// # Errors
/// when could not get cargo folders
pub fn run_all_in_folder(root_folder: &Path) -> Result<Vec<RunResults>> {
    let cargo_projects = utils::get_cargo_folders(root_folder)?;
    let mut results = vec![];

    for project in cargo_projects {
        if let Some(res) = run(&project) {
            results.push(res);
        }
    }
    Ok(results)
}

/// Run the entire CI flow on the given folder path.
///
/// Returns `None` if it is not a Rust folder.
#[must_use]
pub fn run(dir: &Path) -> Option<RunResults> {
    if dir.join("Cargo.toml").exists() {
        Some(RunResults {
            path: dir.to_path_buf(),
            fmt: cargo_fmt(dir).is_ok(),
            clippy: cargo_clippy(dir).is_ok(),
            test: cargo_test(dir, false).is_ok(),
        })
    } else {
        None
    }
}

/// Run cargo test on the given directory.
pub fn cargo_test(dir: &Path, serial: bool) -> Result<Output> {
    let mut params = FMT_TEST.to_vec();
    if serial {
        params.push("--");
        params.push("--test-threads");
        params.push("1");
    }
    println!(
        "Running `cargo {}` in folder {}",
        params.join(" "),
        dir.display()
    );
    Ok(cmd("cargo", params.as_slice()).dir(dir).run()?)
}

/// Run cargo fmt on the given directory.
pub fn cargo_fmt(dir: &Path) -> Result<Output> {
    println!(
        "Running `cargo {}` in folder {}",
        FMT_ARGS.join(" "),
        dir.display()
    );
    Ok(cmd("cargo", FMT_ARGS.as_slice()).dir(dir).run()?)
}

/// Run cargo clippy on the given directory.
pub fn cargo_clippy(dir: &Path) -> Result<Output> {
    println!(
        "Running `cargo {}` in folder {}",
        FMT_CLIPPY.join(" "),
        dir.display()
    );
    Ok(cmd("cargo", FMT_CLIPPY.as_slice()).dir(dir).run()?)
}
