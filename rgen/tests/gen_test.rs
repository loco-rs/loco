use std::fs;

use fs_extra::{self, dir::CopyOptions};
use rgen::{generate, ConsolePrinter, FsDriver, Printer, RealFsDriver};
use serde_json::json;

#[test]
fn test_generate() {
    let FROM = "tests/fixtures/test1/app";
    let GENERATED = "tests/fixtures/test1/generated";

    let fs = Box::new(RealFsDriver {}) as Box<dyn FsDriver>;
    let printer = Box::new(ConsolePrinter {}) as Box<dyn Printer>;
    let vars = json!({"name": "post"});
    fs_extra::dir::remove(GENERATED).unwrap();
    fs_extra::dir::copy(
        FROM,
        GENERATED,
        &CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )
    .unwrap();

    generate(
        &fs,
        &printer,
        &fs::read_to_string("tests/fixtures/test1/template.t").unwrap(),
        &vars,
    )
    .unwrap();
    assert!(!dir_diff::is_different(GENERATED, "tests/fixtures/test1/expected").unwrap());
}

#[test]
fn test_realistic() {
    let FROM = "tests/fixtures/realistic/app";
    let GENERATED = "tests/fixtures/realistic/generated";

    let fs = Box::new(RealFsDriver {}) as Box<dyn FsDriver>;
    let printer = Box::new(ConsolePrinter {}) as Box<dyn Printer>;
    let vars = json!({"name": "email_stats"});
    fs_extra::dir::remove(GENERATED).unwrap();
    fs_extra::dir::copy(
        FROM,
        GENERATED,
        &CopyOptions {
            copy_inside: true,
            ..Default::default()
        },
    )
    .unwrap();

    generate(
        &fs,
        &printer,
        &fs::read_to_string("tests/fixtures/realistic/controller.t").unwrap(),
        &vars,
    )
    .unwrap();
    generate(
        &fs,
        &printer,
        &fs::read_to_string("tests/fixtures/realistic/task.t").unwrap(),
        &vars,
    )
    .unwrap();
    assert!(!dir_diff::is_different(GENERATED, "tests/fixtures/realistic/expected").unwrap());
}
