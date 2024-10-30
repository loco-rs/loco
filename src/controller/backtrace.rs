use std::sync::OnceLock;

use regex::Regex;

use crate::{Error, Result};

static NAME_BLOCKLIST: OnceLock<Vec<Regex>> = OnceLock::new();
static FILE_BLOCKLIST: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_name_blocklist() -> &'static Vec<Regex> {
    NAME_BLOCKLIST.get_or_init(|| {
        [
            "^___rust_try",
            "^__pthread",
            "^__clone",
            "^<loco_rs::errors::Error as",
            "^loco_rs::errors::Error::bt",
            /*
            "^<?tokio",
            "^<?future",
            "^<?tower",
            "^<?futures",
            "^<?hyper",
            "^<?axum",
            "<F as futures_core",
            "^<F as axum::",
            "^<?std::panic",
            "^<?core::",
            "^rust_panic",
            "^rayon",
            "^rust_begin_unwind",
            "^start_thread",
            "^call_once",
            "^catch_unwind",
            */
        ]
        .iter()
        .map(|s| Regex::new(s).unwrap())
        .collect::<Vec<_>>()
    })
}

fn get_file_blocklist() -> &'static Vec<Regex> {
    FILE_BLOCKLIST.get_or_init(|| {
        [
            "axum-.*$",
            "tower-.*$",
            "hyper-.*$",
            "tokio-.*$",
            "futures-.*$",
            "^/rustc",
        ]
        .iter()
        .map(|s| Regex::new(s).unwrap())
        .collect::<Vec<_>>()
    })
}

pub fn print_backtrace(bt: &std::backtrace::Backtrace) -> Result<()> {
    backtrace_printer::print_backtrace(
        &mut std::io::stdout(),
        bt,
        &get_name_blocklist(),
        &get_file_blocklist(),
    )
    .map_err(Error::msg)
}
