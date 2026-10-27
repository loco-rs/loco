//! Production defaults for `SQLite` connections.
//!
//! Most PRAGMAs (`synchronous`, `busy_timeout`, `foreign_keys`, `cache_size`,
//! `mmap_size`) belong to a *connection*, not to the database file, so running
//! them once after the pool is up leaves every other pooled connection on
//! `SQLite`'s defaults. They are therefore set on the connect options, which
//! sqlx applies to every connection it opens.
use std::collections::BTreeMap;

use sqlx::sqlite::SqliteConnectOptions;

/// The PRAGMAs Loco sets unless the config overrides them. Same values Rails 8
/// uses for `SQLite`.
pub(crate) const DEFAULTS: [(&str, &str); 7] = [
    ("journal_mode", "WAL"),
    ("synchronous", "NORMAL"),
    ("busy_timeout", "5000"),
    ("foreign_keys", "ON"),
    ("journal_size_limit", "67108864"),
    ("mmap_size", "134217728"),
    ("cache_size", "2000"),
];

fn is_in_memory(uri: &str) -> bool {
    uri.contains(":memory:") || uri.contains("mode=memory")
}

/// Rejects PRAGMA names that are not plain identifiers.
///
/// # Errors
///
/// On the first invalid name.
pub(crate) fn validate(overrides: &BTreeMap<String, String>) -> crate::Result<()> {
    for name in overrides.keys() {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(crate::Error::string(&format!(
                "invalid SQLite pragma name `{name}`"
            )));
        }
    }
    Ok(())
}

/// Applies the defaults, then `overrides`, to `opts`. WAL is skipped for
/// in-memory databases, where it does nothing. Call [`validate`] first.
pub(crate) fn apply(
    mut opts: SqliteConnectOptions,
    uri: &str,
    overrides: &BTreeMap<String, String>,
) -> SqliteConnectOptions {
    let in_memory = is_in_memory(uri);
    for (name, value) in DEFAULTS {
        if in_memory && name == "journal_mode" {
            continue;
        }
        opts = opts.pragma(name, value);
    }
    for (name, value) in overrides {
        opts = opts.pragma(name.clone(), value.clone());
    }
    opts
}
