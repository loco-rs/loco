+++
title = "Data"
description = ""
date = 2024-02-07T08:00:00+00:00
updated = 2024-02-07T08:00:00+00:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

`Loco` provides a simple static data loader facility. This can be useful for the following cases:

* You need access to read-only data that has to be loaded from a JSON file
* You download data from external sources periodically, and want to use it in your process (to refresh you typically restart the process or read from disk directly)


Examples:

* Machine learning model hyperparameters (that are updated from time to time)
* IP banlist
* Calendar-related events
* Stock data
* Security policies
* Per-container policies or configuration

## Creating a new data loader

Use the `data` generator:

```
$ cargo loco g data stocks
added: "data/stocks/data.json"
added: "src/data/stocks.rs"
injected: "src/data/mod.rs"
* Data loader `Stocks` was added successfully.
```

The actual data should be placed in the new `data/` folder (next to `src/`). Similar to how configuration is placed in `config/`. Here, the JSON data file is named `data/stocks/data.json`.

The data _module_ is in the `src/data/stocks.rs` module that was added, and creates a new `crate::data::stocks` namespace available statically from anywhere in your code.

Remember, to load the data your app _binary_ needs to see a `data/` folder next to it. If you want to customize the name of this folder you can set the `LOCO_DATA` environment variable.

## Shape your data structure

Your `src/data/stocks.rs` file contains an initial definition for the data which was automatically generated:

```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stocks {
    pub is_loaded: bool,
}
```

When you put your real data in `data/stocks/data.json` you should define the _shape_ of your data in the `Stocks` struct to match (you can do this automatically with a tool like [quicktype](https://quicktype.io/)). You can use any `serde`-friendly data type.


## Using your static data

Use `data::stocks::get()` from anywhere to access the data which is loaded **once** for the duration of the life of your process (this will use an in-memory image of your data). You can call `get()` as many times as you want and pay no special performance fee for it.

Use `data::stocks::read()`  to read directly from disk (note: this will spend IO time reading for every call).

## Updating the process data

Because this data is loaded **once** for the duration of the life of your process, you need to restart your process to effectively update it. 

For the `data` subsystem we assume that the use cases around these types of data is massively read many more times than it is updated (but it is updated from time to time), so it is a read-heavy use case, and data that is _frequently_ updated in any case needs a different storage paradigm (cache, database, etc.). The in-memory copy of your data will have the best read access performance possible, like any other static data.

In cases you do need to update this data, restarting a Loco process is _fast_, and is similar in concept to deploying a new version, but not deploying new code which saves time and effort.

You can also use the `read()` function to read from disk, and cache it somewhere centrally (you can use the Loco `cache` system).
