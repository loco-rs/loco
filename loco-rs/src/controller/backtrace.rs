use lazy_static::lazy_static;
use regex::Regex;

use crate::{Error, Result};

lazy_static! {
    static ref NAME_BLOCKLIST: Vec<Regex> = [
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
    .collect::<Vec<_>>();
    static ref FILE_BLOCKLIST: Vec<Regex> = ["axum-.*$", "tower-.*$", "hyper-.*$", "tokio-.*$", "futures-.*$", "^/rustc"]
        .iter()
        .map(|s| Regex::new(s).unwrap())
        .collect::<Vec<_>>();
}

pub fn print_backtrace(bt: &std::backtrace::Backtrace) -> Result<()> {
    backtrace_printer::print_backtrace(&mut std::io::stdout(), bt, &NAME_BLOCKLIST, &FILE_BLOCKLIST)
        .map_err(Error::msg)
}
