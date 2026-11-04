use std::{env, fs::OpenOptions, io::Write};

fn main() {
    let arguments: Vec<_> = env::args().skip(1).collect();
    assert_eq!(arguments[0], "task");
    assert_eq!(arguments[1], "foo");
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&arguments[2])
        .unwrap();
    // One write call per line, so the test never reads a half-written line.
    let line = format!(
        "{:?}|{}|{}\n",
        &arguments[3..],
        env::var("LOCO_ENV").unwrap(),
        env::var_os("PATH").is_some()
    );
    output.write_all(line.as_bytes()).unwrap();
}
