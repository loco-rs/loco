+++
title = "Load static data"
description = "Load read-only JSON data (hyperparameters, banlists, calendars) once per process using loco_rs::data, without a database round trip."
date = 2026-07-03T00:00:00+00:00
updated = 2026-07-03T00:00:00+00:00
draft = false
weight = 34
sort_by = "weight"
template = "docs/page.html"
aliases = ["/docs/infrastructure/data/"]

[extra]
lead = ""
toc = true
top = false
+++

Goal: give your app access to read-only data that lives in a JSON file — loaded once and kept in memory — without standing up a database table or hand-rolling file I/O.

This is a good fit for data that's read far more often than it changes: machine learning hyperparameters, an IP banlist, calendar events, stock data, security policies, per-container configuration. If the data changes on every request, reach for the [cache](@/docs/how-to/use-cache.md) or the database instead.

## 1. Generate a data loader

```
$ cargo loco g data stocks
added: "data/stocks/data.json"
added: "src/data/stocks.rs"
injected: "src/data/mod.rs"
* Data loader `Stocks` was added successfully.
```

This creates a `data/stocks/data.json` file (next to `src/`, the same way `config/` sits next to `src/`) and a `src/data/stocks.rs` module under the `crate::data::stocks` namespace.

## 2. Shape your data

`src/data/stocks.rs` starts with a placeholder struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stocks {
    pub is_loaded: bool,
}
```

Replace it with a struct matching the real shape of `data/stocks/data.json` (tools like [quicktype](https://quicktype.io/) can generate this from a sample file). Any `serde`-friendly type works.

## 3. Load the data

Under the hood, the generated module calls into `loco_rs::data`, which exposes two loader functions:

```rust
// asynchronous — use from controllers, workers, tasks
pub async fn load_json_file<T: DeserializeOwned>(path: &str) -> Result<T>;

// synchronous — use during boot, or outside an async context
pub fn load_json_file_sync<T: DeserializeOwned>(path: &str) -> Result<T>;
```

Both resolve `path` relative to a data folder — `data/` by default. Call `data::stocks::get()` from anywhere to read the in-memory copy, loaded once for the life of the process; it's cheap to call as many times as you like. Call `data::stocks::read()` if you instead want to re-read straight from disk on every call (this pays an I/O cost each time).

## 4. Point at a different data folder (optional)

The data folder defaults to `data/`, resolved relative to wherever the app binary runs. Set the `LOCO_DATA` environment variable to override it:

```
LOCO_DATA=/etc/myapp/data cargo loco start
```

Whatever machine runs the binary needs a `data/` folder (or the `LOCO_DATA` path) present alongside it, the same way it needs `config/`.

## Updating the data

Because the in-memory copy is loaded once per process, picking up new data means restarting the process — conceptually similar to a deploy, but without rebuilding or shipping new code, so it's fast. If you need to refresh without a restart, call the `read()` function directly and cache the result yourself using the [cache](@/docs/how-to/use-cache.md) system.
